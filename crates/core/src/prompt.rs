//! Turning a diagram and its review comments into text for an LLM: a brief
//! that describes the diagram, and prompts that ask an agent to act on the
//! open comments, either by editing the diagram or by changing code.
use serde::{Deserialize, Serialize};

use crate::dsl;
use crate::model::*;

/// What the agent should do with the comments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// Change the diagram (respond with ops or a new diagram).
    #[default]
    Diagram,
    /// Change the codebase the diagram describes.
    Code,
    /// Answer questions / give feedback, without changing anything.
    Discuss,
}

/// How the agent should express diagram changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reply {
    /// `{"ops": [...]}` — precise, keeps everything else untouched.
    #[default]
    Ops,
    /// The whole updated diagram in the DSL.
    Dsl,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PromptOptions {
    pub target: Target,
    pub reply: Reply,
    /// Only these comments (all open ones when empty).
    pub comments: Vec<String>,
    /// Also include resolved comments (as context).
    pub include_resolved: bool,
    /// Extra instructions appended to the prompt.
    pub instructions: Option<String>,
    /// Path of the diagram file, so code agents can update it.
    pub file: Option<String>,
    /// How much of the board the prompt shows.
    pub context: Context,
}

/// How much of the board a prompt about comments shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Context {
    /// The whole board when it is small, else the commented sections.
    #[default]
    Auto,
    /// What the comments are on, their arrows and the blocks at the other ends.
    Near,
    /// The sections (or swimlanes, charts, maps) the commented blocks are in.
    Section,
    /// The whole board.
    All,
}

/// Boards up to this many lines go into prompts whole (`Context::Auto`).
const SMALL_BOARD: usize = 60;

/// The part of the board a prompt shows: what the comments are on, and
/// with `Section`, the blocks around it. None: the whole board.
fn context_ids(doc: &Document, comments: &[&Comment], context: Context) -> Option<Vec<String>> {
    let context = match context {
        Context::Auto if dsl::print_lean(doc).lines().count() <= SMALL_BOARD => return None,
        Context::Auto => Context::Section,
        Context::All => return None,
        c => c,
    };
    let mut sels: Vec<String> = Vec::new();
    for c in comments {
        // a comment on the canvas is about the whole board
        let on = c.on.as_deref()?;
        if let Some(e) = doc.edge(on) {
            sels.push(format!("near:{}", e.from));
            sels.push(format!("near:{}", e.to));
            continue;
        }
        sels.push(format!("near:{on}"));
        if context == Context::Section {
            // the block it sits in: a map, a lane's swimlanes, a section
            let n = doc.node(on);
            let mut root = n;
            while let Some(p) = root
                .and_then(|x| x.parent.as_deref())
                .and_then(|p| doc.node(p))
            {
                root = Some(p);
            }
            if let Some(r) =
                root.filter(|r| r.mindmap.is_some() || n.is_some_and(|n| n.parent.is_some()))
            {
                sels.push(format!("map:{}", r.id));
            }
            let mut g = n
                .and_then(|n| root.unwrap_or(n).group.as_deref())
                .or(doc.group(on).map(|g| g.id.as_str()));
            if let Some(lanes) = g
                .and_then(|g| doc.group(g))
                .and_then(|g| g.parent.as_deref())
                .filter(|p| doc.group(p).is_some_and(|p| p.lanes))
            {
                g = Some(lanes);
            }
            if let Some(g) = g {
                sels.push(format!("group:{g}"));
            }
        }
    }
    let sels: Vec<&str> = sels.iter().map(String::as_str).collect();
    let mut ids = crate::query::select(doc, &sels).ok()?;
    ids.extend(comments.iter().map(|c| c.id.clone()));
    Some(ids)
}

pub const OPS_REFERENCE: &str = r#"Operations (fields in [] are optional; ids are letters, digits, `_`, `-`, `.`):
- {"op":"add_node","id":"cache","label":"Redis","shape":"cylinder",["color","detail","group","link","tags","x","y","w","h","parent","index","mindmap"]}
- {"op":"update_node","id":"api",["label","shape","color","detail","group","link","tags","w","h","parent","index","mindmap","collapsed","start","days","after","milestone","done","owner"]}   ("" clears a field)
- {"op":"add_outline","parent":"risks","outline":"Fraud\n  Card testing\nBank delays"}   (mindmap topics, two spaces per level, a line may end in [id: x, red]; without parent: a new mindmap, "group" puts it in a section)
- {"op":"add_edge","from":"api","to":"cache",["id","label","arrow":"end|start|both|none","line":"solid|dashed|dotted|thick","color"]}
- {"op":"update_edge","id":"api->cache",["from","to","label","arrow","line","color"]}
- {"op":"add_group","id":"backend","label":"Backend",["color","parent","members":["api","db"]]}
- {"op":"update_group","id":"backend",["label","color","parent"]}
- {"op":"rename","id":"api","to":"orders-api"}            (updates edges and comments)
- {"op":"remove","id":"cache"}                            (node, group, edge or comment; a node's edges go too; a mindmap topic takes its branch)
- {"op":"move","id":"api","x":120,"y":40}  {"op":"unpin"[, "id":"api"]}
- {"op":"add_comment","text":"…",["on":"api","author"]}  {"op":"reply","id":"c1","text":"…",["author"]}
- {"op":"resolve_comment","id":"c1",["resolution":"what was done"]}  {"op":"reopen_comment","id":"c1"}
- {"op":"set","key":"title|description|type|direction|layout|routing|theme","value":"…"}   (type: sketch, product or project, which decides the built-in perspectives; theme: light, dark, a preset — paper, blueprint, midnight, contrast — or a theme's id)
- {"op":"perspective","id":"exec",["label","tags":["exec"],"show":["adr","risks"],"details":false,"remove":true]}   (a way of looking at the board: elements with one of its tags, blocks of the kinds in show; elements carry tags such as tech, product, user, exec, team)
- {"op":"presentation",["title","transition":"glide|fade|zoom|none","instructions","slides":[{"target":"risks",["title","notes","transition"]}],"remove":true]}   (the board as slides, in order; a target is a top-level section, block, mindmap centre or box, or "*" for the whole board; slides replaces the whole list; notes are what to say)
- {"op":"theme","source":"theme brand {\n  blue: #3b6fd8\n  light {\n    background: #faf9f6\n  }\n  dark {\n    background: #101418\n  }\n}"}   (adds or replaces a theme; top lines apply in both modes, light { } and dark { } what differs; keys: background, dots, text, muted, surface, border, line, section, header, title, title-text, pin, font, radius and the named colours)
- {"op":"merge","dsl":"cache: Redis [cylinder]\napi -> cache: reads\napi [red]"}   (statements in the Shape language, as if written at the end of the file: adds new elements; one written again by its id gets the attributes given. Cheapest for many changes)
- Database table: {"op":"add_group","id":"users","label":"Users","entity":true}; its columns are add_node / update_node with "id":"users.email","label":"email","group":"users","column":{"type":"text",["pk":true,"unique":true,"null":true,"fk":"teams.id","default":"now()"]}. Or merge: "entity users: Users {\n  id: uuid [pk]\n  team_id: uuid [fk: teams]\n}"
- Database schema: a section of tables from a schema file ({"op":"add_group","id":"db","label":"Database","schema":true,"file":"db/structure.sql"}). Its tables come from that file: change the file, not them (`shape schema BOARD` updates them).
- Forecast: merge "forecast revenue: Revenue [unit: £] {\n  now: 12k\n  Phase 1 GA [adds: 3k, ramp: 3]\n}" (a launch is a gantt milestone by its name, or `on: DATE`); ops: add_group with "forecast":true,"now":12000, its launches add_node with "group":"revenue","label":"Phase 1 GA","adds":3000,"ramp":3.
- Canvases: a box with "canvas":"payments" opens a board inside this one (not shown to you; keep the attribute when you rewrite a box). {"op":"add_canvas","title":"Payments"} adds one; removing the box that opens it removes it. {"op":"in","canvas":"payments","ops":[…]} applies ops inside canvas `payments` (`payments/fraud` for one inside it).
- Images: a box with "image":"img1" shows asset img1 (its data is not shown to you); keep the image attribute when you rewrite a box.
- Sequence diagram: {"op":"add_group","id":"login","label":"Sign in","sequence":true}; participants are nodes in it, messages are add_edge between them (in order; {"op":"update_edge","id":"web->api","index":0} moves one). Or merge: "sequence login: Sign in {\n  user: User\n  web: Web app\n  user -> web: opens /login\n  web ..> user: page\n}" (..> is a reply)
- Decision record: {"op":"add_group","id":"db","label":"Use Postgres","adr":"proposed"} (status: proposed, accepted, rejected, superseded, deprecated); its fields are add_node / update_node with "id":"db.context","label":"context","group":"db","detail":"the text". Or merge: "adr db: Use Postgres [accepted] {\n  context: …\n  decision: …\n}"
- Kanban: {"op":"add_group","id":"sprint","label":"Sprint 12","kanban":true}, its columns {"op":"add_group","id":"doing","label":"Doing","parent":"sprint"}; a card is a node in a column; move one with {"op":"update_node","id":"login","group":"doing","index":0}. Or merge: "kanban sprint: Sprint 12 {\n  column todo: To do {\n    login: Login page\n  }\n  column doing: Doing {}\n}"
- Cycle: {"op":"add_group","id":"loop","label":"Build loop","cycle":true}; its steps are add_node with "group":"loop", in order. Or merge: "cycle loop: Build loop {\n  Plan\n  Build\n  Measure\n}"
- To-do list: merge "todo launch: Launch checklist {\n  [x] Release notes [owner: ana]\n  [ ] Rollback tested [due: 2026-10-28]\n    [ ] On staging\n}" (two spaces more for a sub-item). Tick one with {"op":"update_node","id":"launch.release-notes","done":100} (0 unticks); add one with add_node "group":"launch" (["level":1] for a sub-item, "index" for its place), owner and due date as "owner" and "date".
- Wireframe screen: merge "screen login: Sign in [mobile] {\n  header: Acme\n  title: Welcome back\n  input email: Email\n  password: Password\n  button submit: Sign in [blue]\n  link forgot: Forgot password?\n}" (device: mobile, tablet or web). One component per line, `kind [id]: Label [attrs]`; its id is `login.submit`. Kinds: header, navbar, title, subtitle, text, button (a colour fills it), buttons, link, input, password, search, textarea, select, date, checkbox, radio, toggle, slider, image, video, map, chart, avatar, card (detail = its text), list, table, tabs, tabbar, sidebar, breadcrumb, badges, alert, progress, stepper, pagination, rating, divider, spacer, footer, fab. Items of list, tabs, tabbar, navbar, sidebar, buttons, radio, badges, stepper, breadcrumb, table (its columns) go in the label separated by ` | `. A flow is an arrow from a component to the next screen: `login.submit -> home: signs in`. Or ops: add_group with "screen":"mobile", add_node with "group":"login","widget":"button".
- Timeline: {"op":"add_group","id":"history","label":"History","timeline":true}; its events are add_node / update_node with "group":"history","label":"Open beta","date":"2026-10-28" (also "Oct 2026", "Q4 2026", "2026", or a span "7–14 May 2026") and ["important":true,"color","detail"]. Or merge: "timeline history: History {\n  2026-10-28: Open beta [important]\n  Dec 2024: Supplier deal\n}"
- Table: {"op":"add_group","id":"opts","label":"Options","columns":["option","cost","notes"]}; its rows are add_node / update_node with "group":"opts","label":"Local" (the first cell),"cells":["$0","needs a GPU"]. Or merge: "table opts: Options {\n  option | cost | notes\n  Local | $0 | needs a GPU\n}"
- Chart: {"op":"add_group","id":"rev","label":"Revenue","chart":"bar|line|area|pie|donut|scatter",["stacked":true,"horizontal":true,"unit":"%","series":["EU","US"],"axis":"quarter"]}; its rows are add_node / update_node with "group":"rev","label":"Q1","values":[120,80] (one per series, null for none). Or simpler, merge: "chart rev: Revenue [bar] {\n  quarter | EU | US\n  Q1 | 120 | 80\n}"
- Risk matrix: {"op":"add_group","id":"risks","label":"Risks","risks":true,["levels":3|4|5]}; its risks are add_node / update_node with "group":"risks" and ["probability":1-5,"impact":1-5,"owner","detail" (the description)]
- Gantt: a chart is {"op":"add_group","id":"plan","label":"Plan","gantt":true,["start":"YYYY-MM-DD","scale":"day|week|month|quarter"]}; its tasks are add_node / update_node with "group":"plan" and ["start","days","after":["id"],"milestone":true,"done":60,"owner":"ana"] (after may name tasks in other charts)
Shapes: box, rounded, pill, circle, diamond, hexagon, cylinder, parallelogram, document, person, cloud, sticky, text.
Colours: gray, blue, green, yellow, orange, red, purple, pink, teal or #rrggbb.
Edge ids default to `from->to` (`from->to#2` for a second edge between the same nodes).
Mindmaps: a topic's `parent` is the topic it branches from (`index` = position among siblings); a centre has `mindmap`: right|left|both|down. Prefer add_outline for new topics: it costs the fewest tokens."#;

fn quote_block(text: &str) -> String {
    text.lines().map(|l| format!("   > {l}\n")).collect()
}

fn comment_md(doc: &Document, c: &Comment, n: usize) -> String {
    let target = match &c.on {
        Some(on) => format!("on {}", doc.describe(on)),
        None => "on the whole diagram".to_string(),
    };
    let by = c
        .author
        .as_ref()
        .map(|a| format!(" — @{a}"))
        .unwrap_or_default();
    let status = if c.status == Status::Resolved {
        " (resolved)"
    } else {
        ""
    };
    let mut s = format!(
        "{n}. **{}** {target}{by}{status}\n{}",
        c.id,
        quote_block(&c.text)
    );
    for r in &c.replies {
        let by = r
            .author
            .as_ref()
            .map(|a| format!("@{a}: "))
            .unwrap_or_default();
        s.push_str(&format!("   - {by}{}\n", r.text.replace('\n', " ")));
    }
    if let Some(res) = &c.resolution {
        s.push_str(&format!("   - resolution: {}\n", res.replace('\n', " ")));
    }
    s
}

/// A prompt asking a model to change the board as `request` says. With
/// `ids` (a selection), it shows that part of the board (and what is around
/// it); otherwise the whole board. The model replies with ops.
pub fn ask(
    doc: &Document,
    ids: &[String],
    request: &str,
    recipes: &[crate::recipes::Recipe],
) -> String {
    let title = doc.title.as_deref().unwrap_or("Untitled diagram");
    let mut s = format!("# Change the diagram \"{title}\"\n\n");
    s.push_str("The diagram is written in the `shape` language: one statement per line (`id: Label [attrs]` declares a box, `a -> b: label` connects two, `group id: Label { … }` is a section; `mindmap`, `gantt`, `chart`, `table` and `risks` blocks hold their items one per line). The request below either gives information to add (facts, details, notes: put each where it belongs, in or next to what it is about, or as a detail of it) or asks for a change; do what it says.\n\n");
    if let Some(d) = &doc.description {
        s.push_str(&format!("About the diagram: {d}\n\n"));
    }
    // the selection and what is around it, or everything
    let part = (!ids.is_empty())
        .then(|| {
            let sels: Vec<String> = ids
                .iter()
                .flat_map(|id| {
                    let mut v = vec![id.clone()];
                    if doc.node(id).is_some() {
                        v.push(format!("near:{id}"));
                    }
                    v
                })
                .collect();
            let sels: Vec<&str> = sels.iter().map(String::as_str).collect();
            crate::query::select(doc, &sels).ok()
        })
        .flatten();
    match part {
        Some(picked) => {
            let sub = crate::query::subset(doc, &picked);
            let shown = sub.nodes.len() + sub.edges.len() + sub.groups.len();
            let all = doc.nodes.len() + doc.edges.len() + doc.groups.len();
            s.push_str(&format!(
                "## The part of the board this is about\n\nSelected: {}. Shown with what is around it ({shown} of the board's {all} elements); the rest exists too: keep its ids, do not remove or recreate it.\n\n```shape\n{}```\n\n",
                ids.iter().map(|i| format!("`{i}`")).collect::<Vec<_>>().join(", "),
                dsl::print_lean(&sub)
            ));
        }
        None => s.push_str(&format!(
            "## The board\n\n```shape\n{}```\n\n",
            dsl::print_lean(doc)
        )),
    }
    s.push_str(&format!("## The request\n\n{}\n\n", request.trim()));
    s.push_str(BEST_BLOCKS);
    s.push_str(&crate::recipes::prompt_section(recipes));
    s.push('\n');
    s.push_str("## How to respond\n\nReply with a single ```json code block holding the operations, for example:\n\n```json\n{\"ops\": [\n  {\"op\": \"update_node\", \"id\": \"api\", \"color\": \"red\"},\n  {\"op\": \"merge\", \"dsl\": \"cache: Redis [cylinder]\\napi -> cache: reads\"}\n]}\n```\n\n");
    s.push_str(OPS_REFERENCE);
    s.push_str("\n\nRules:\n- Change only what the request asks for; keep existing ids.\n- Information that fits an existing item goes into it (its label or `detail`); new information goes into the block that shows it best (dates on a timeline, tasks on a to-do list, a risk on the matrix…), or that block is added; a box or sticky only for what fits none.\n- To add several things, prefer one `merge` with statements in the Shape language.\n- The batch is applied all at once or not at all, so it must be valid as a whole.\n- No explanation is needed outside the code block.\n");
    s
}

/// The whole board for a prompt: the top board, then every canvas inside
/// it (`## Canvas `path` (“Title”)`), each as a brief with every id.
fn board_by_canvas(root: &Document, today: Option<i64>) -> String {
    fn walk(s: &mut String, doc: &Document, path: &[String], title: &str, today: Option<i64>) {
        let at = if path.is_empty() {
            "## The top board".to_string()
        } else {
            format!("## Canvas `{}` (“{title}”)", path.join("/"))
        };
        let body = with_today(today, || brief(doc));
        // brief's own title line would repeat the heading
        let body = body.split_once('\n').map(|(_, b)| b).unwrap_or(&body);
        s.push_str(&format!("{at}\n{}\n\n", body.trim_end()));
        for c in &doc.canvases {
            let mut p = path.to_vec();
            p.push(c.id.clone());
            walk(s, &c.doc, &p, c.display(), today);
        }
    }
    let mut s = String::new();
    let title = root.title.as_deref().unwrap_or("Untitled diagram");
    walk(&mut s, root, &[], title, today);
    s
}

/// Every canvas of `root` with its path, the top board first.
fn canvases_of(root: &Document) -> Vec<(Vec<String>, &Document)> {
    fn walk<'a>(doc: &'a Document, path: Vec<String>, out: &mut Vec<(Vec<String>, &'a Document)>) {
        out.push((path.clone(), doc));
        for c in &doc.canvases {
            let mut p = path.clone();
            p.push(c.id.clone());
            walk(&c.doc, p, out);
        }
    }
    let mut out = Vec::new();
    walk(root, Vec::new(), &mut out);
    out
}

/// What Rethink does unless the person asks for something else: the
/// editable part of [`rethink`]'s prompt (the editor shows it to change).
/// Which block shows which kind of information best: what Rethink goes by
/// when it organises a board.
pub const BEST_BLOCKS: &str = "## The right block for each kind of information

Pick the block that lets someone get it at a glance; plain boxes are for parts of a system and their connections.
- What happened when, and the dates ahead: `timeline` (`2026-10-28: Open beta [important]`).
- Work to do, with owners and due dates: `todo` (`[ ] Item [owner: ana, due: 28 Oct]`); work moving through stages: `kanban` (`column` blocks of cards).
- A plan with durations and dependencies: `gantt` (`Task [2w, after: design, done: 40%]`, `[milestone]` for the dates that matter); its forecast and the Delivery summary follow from it.
- What could go wrong: `risks` (`Risk [p: 4, i: 3, owner: sam]`); something that already blocks work: a card in a blockers `kanban` (column `Blocked`/`Open`).
- A choice made or to make, with its options: `adr` (`context:`, `options:`, `decision:`, `consequences:`, `[proposed]` until decided).
- Numbers over time or by category: `chart`; side-by-side facts (options, who does what): `table`; expected revenue as launches ship: `forecast` (`now:`, a milestone per line with `adds:` and `ramp:`).
- Ideas, a problem broken down, scope: `mindmap`; a loop that repeats: `cycle`.
- Who does which step, or how a request travels: `lanes` (swimlanes) or `sequence`; screens and the flow between them: `screen` (wireframes); database tables: `entity` (or a `schema` read from a file).
- Each line's exact form is in the language reference; when unsure, keep the information as it is rather than force it into a block.

";

pub const RETHINK_DEFAULT: &str = "Bring this canvas up to date and make it easy to read.

1. Act on every open comment: change the board as it says, wherever it matters (one new fact can change items on other canvases too), then resolve it with a short resolution saying what changed. If you cannot act on one, reply saying what is missing and leave it open.
2. Take in what is new (recent changes, loose stickies, notes) and revise the older items around it: update what it changes, and mark what it supersedes instead of deleting it.
3. Organise: put related items in clearly named sections, merge duplicates into one item that keeps everything they said, and fold settled discussion and stray notes into the items they are about.
4. Show each kind of information in the block that communicates it best (see below): loose boxes, stickies and text that are really dates, tasks, a plan, risks, a decision, numbers or screens read better as a timeline, a to-do list, a gantt chart, a risk matrix, a decision record, a chart or a wireframe. Move them in, then remove the loose ones.
5. Give every item a short, specific title, and put the context the title leaves out in its details.
6. Fix arrows that no longer make sense, and add the ones that are missing.

Keep every piece of information: reorganise, merge and move it, but never drop it.";

/// A prompt asking a model to rethink the whole board (`root`, every canvas
/// included): act on the open comments, then organise what has been
/// collected so it reads well now. `focus` is the canvas the person is
/// looking at. With `since` (an earlier version of the board and what that
/// version is, e.g. "where this branch left main"), what changed since is
/// the new information to revise the rest around. The model replies with a
/// note and ops (see [`reply_note`]); ops for a canvas go in an `in` op.
pub fn rethink(
    root: &Document,
    focus: &[String],
    since: Option<(&Document, &str)>,
    instructions: Option<&str>,
    recipes: &[crate::recipes::Recipe],
) -> String {
    let title = root.title.as_deref().unwrap_or("Untitled diagram");
    let mut s = format!("# Rethink the board \"{title}\"\n\n");
    s.push_str("People have been collecting information on this board: findings, ideas, notes, decisions, plans and comments, added one by one as they came. Bring it up to date with everything on it, then organise it so it reads well now.\n\n");
    s.push_str("The board is written in the `shape` language: one statement per line (`id: Label [attrs]` declares a box, `a -> b: label` connects two, `group id: Label { … }` is a section; `mindmap`, `gantt`, `chart`, `table` and `risks` blocks hold their items one per line; `comment` lines are what people said). Below are the top board and every canvas inside it (a board inside the board, opened from a box with `canvas: ID`).\n\n");
    if let Some(d) = &root.description {
        s.push_str(&format!("About the board: {d}\n\n"));
    }
    if !focus.is_empty() {
        s.push_str(&format!("The person asking is looking at canvas `{}`; change whatever the information affects, there and anywhere else.\n\n", focus.join("/")));
    }
    s.push_str(&board_by_canvas(root, None));
    // open comments: what people asked for or told the board
    let open: Vec<String> = canvases_of(root)
        .into_iter()
        .flat_map(|(path, doc)| {
            doc.comments
                .iter()
                .filter(|c| c.status == Status::Open)
                .map(move |c| {
                    let at = if path.is_empty() {
                        "on the top board".to_string()
                    } else {
                        format!("in canvas `{}`", path.join("/"))
                    };
                    let on =
                        c.on.as_deref()
                            .map(|o| format!(", on {}", doc.describe(o)))
                            .unwrap_or_default();
                    let by = c
                        .author
                        .as_deref()
                        .map(|a| format!(" by {a}"))
                        .unwrap_or_default();
                    let mut line =
                        format!("- `{}` {at}{on}{by}: {}\n", c.id, c.text.replace('\n', " "));
                    for r in &c.replies {
                        let by = r
                            .author
                            .as_deref()
                            .map(|a| format!("{a}: "))
                            .unwrap_or_default();
                        line.push_str(&format!("  - reply, {by}{}\n", r.text.replace('\n', " ")));
                    }
                    line
                })
                .collect::<Vec<_>>()
        })
        .collect();
    if !open.is_empty() {
        s.push_str(&format!("## Open comments\n\nThese are new information and requests from the people on the board. They come first.\n\n{}\n", open.concat()));
    }
    if let Some((old, label)) = since {
        let mut changes = String::new();
        for (path, now) in canvases_of(root) {
            let before = old.at(&path).cloned().unwrap_or_default();
            let d = crate::diff::diff(&before, now);
            if d.is_empty() {
                continue;
            }
            if path.is_empty() {
                changes.push_str("### The top board\n\n");
            } else {
                changes.push_str(&format!("### Canvas `{}`\n\n", path.join("/")));
            }
            changes.push_str(&crate::diff::markdown(&d, &before, now));
            changes.push('\n');
        }
        if !changes.is_empty() {
            s.push_str(&format!("## What is new since {label}\n\nThis is new information too. Revise the older parts of the board around it: what it confirms, changes, contradicts or makes obsolete.\n\n{changes}\n"));
        }
    }
    let what = instructions
        .map(str::trim)
        .filter(|i| !i.is_empty())
        .unwrap_or(RETHINK_DEFAULT);
    s.push_str(&format!("## What to do\n\n{what}\n\n"));
    s.push_str(BEST_BLOCKS);
    s.push_str(&perspectives_section(root));
    s.push_str("How to change the board:\n- Keep ids; to merge, keep one item and remove the others only after moving what they said into it.\n- Ops apply to the top board. For anything inside a canvas, wrap them in an `in` op with the canvas path: `{\"op\": \"in\", \"canvas\": \"plan\", \"ops\": [...]}` (`plan/details` for a canvas inside `plan`). A comment's own ops (`resolve_comment`, `reply`) go in the `in` of the canvas it is in.\n- A box's `detail` holds the context its title leaves out; on a kanban card it shows on hover.\n- Do not set `x`/`y` or move things: the layout follows the structure.\n- Leave the data inside blocks (gantt dates, table cells, chart values, columns) as it is unless the instructions or the information say otherwise.\n- If nothing needs to change, change nothing and say so in the note.\n\n");
    s.push_str(&crate::recipes::prompt_section(recipes));
    s.push('\n');
    s.push_str("## How to respond\n\nReply with a single ```json code block: a short note for the people on the board (one to three sentences: what you changed and why) and the operations, for example:\n\n```json\n{\"note\": \"Split the plans into a free and a team plan as the comment says, priced per seat, and grouped the interview findings by theme.\", \"ops\": [\n  {\"op\": \"add_group\", \"id\": \"pricing\", \"label\": \"Pricing\", \"members\": [\"n3\", \"n7\"]},\n  {\"op\": \"remove\", \"id\": \"n7\"},\n  {\"op\": \"in\", \"canvas\": \"status\", \"ops\": [\n    {\"op\": \"update_node\", \"id\": \"scope\", \"label\": \"Two plans: a free plan and a team plan\"},\n    {\"op\": \"resolve_comment\", \"id\": \"c1\", \"resolution\": \"Scope now lists both plans\"}\n  ]}\n]}\n```\n\n");
    s.push_str(OPS_REFERENCE);
    s.push_str("\n\nThe batch is applied all at once or not at all, so it must be valid as a whole. No explanation is needed outside the code block.\n");
    s
}

/// The board's perspectives, for Rethink: each audience sees what is
/// tagged for it, so a rethought board keeps every perspective filled.
fn perspectives_section(root: &Document) -> String {
    let all = crate::perspectives::all(root);
    if all.is_empty() {
        return String::new();
    }
    let mut s = String::from("## Perspectives\n\nThe board is seen by these audiences. Each sees the elements tagged for it (and blocks of the kinds it shows whole):\n\n");
    for p in &all {
        let tag = p.tags.first().cloned().unwrap_or_else(|| p.id.clone());
        let shown = crate::perspectives::view(root, p).nodes.len();
        let whole = if p.show.is_empty() {
            String::new()
        } else {
            format!(", shows {} blocks whole", p.show.join(", "))
        };
        s.push_str(&format!(
            "- {}: tag `{tag}`{whole}; now shows {shown} elements on the top board\n",
            p.label
        ));
    }
    s.push_str("\nKeep them up to date: give each box the tags of every perspective it matters to (`update_node` with its existing `tags` plus the new ones; a box can belong to several, in canvases too). Tag mindmap centres, not their topics. Every perspective should show what that audience needs, so none is empty.\n\n");
    s
}

/// A prompt asking a model a question about the whole board (`root`, its
/// canvases included), with the conversation so far. The model answers in
/// a few sentences of plain text, pointing at elements as `[[id]]` (top
/// board) or `[[canvas/…/id]]` (inside a canvas); see [`answer_refs`].
pub fn question(
    root: &Document,
    history: &[(String, String)],
    question: &str,
    today: Option<i64>,
) -> String {
    let title = root.title.as_deref().unwrap_or("Untitled diagram");
    let mut s = format!("# A question about the board \"{title}\"\n\nAnswer the question at the end from what is on this board. The board is below: the top board, then each canvas inside it (a board inside the board, opened from a box), each in the `shape` language with every id.\n\n");
    s.push_str(&board_by_canvas(root, today));
    if !history.is_empty() {
        s.push_str("## Earlier in this conversation\n\n");
        for (q, a) in history {
            s.push_str(&format!("Q: {}\nA: {}\n\n", q.trim(), a.trim()));
        }
    }
    s.push_str(&format!("## The question\n\n{}\n\n", question.trim()));
    s.push_str("## How to answer\n\n- Answer in a few sentences or a short list, plainly, without a preamble.\n- Point at the board where you mention something: `[[id]]` for an element of the top board, `[[canvas/id]]` for one inside a canvas (the canvas path, then the id; e.g. `[[plan/sso]]`), `[[canvas]]` for a whole canvas. Use only ids that are on the board; the reader clicks them to go there.\n- When the board does not answer the question, say so, and say what is missing; do not make things up.\n- Plain text: **bold** and `- ` bullets at most; no headings, tables or code blocks.\n");
    s
}

/// A prompt asking a model to answer `question` about the whole board
/// (`root`, canvases included) with a small diagram of its own: a summary
/// in a sentence or two, then a board in the `shape` language whose
/// elements link back to where they come from (`#/id` on the top board,
/// `#/plan/id` inside a canvas). See [`answer_parts`].
pub fn answer_board(
    root: &Document,
    history: &[(String, String)],
    question: &str,
    today: Option<i64>,
    recipes: &[crate::recipes::Recipe],
) -> String {
    let title = root.title.as_deref().unwrap_or("Untitled diagram");
    let mut s = format!("# A question about the board \"{title}\"\n\nAnswer the question at the end by drawing a small board of your own: a diagram made for this question from what is on the board below (the top board, then each canvas inside it, in the `shape` language with every id).\n\n");
    s.push_str(&board_by_canvas(root, today));
    if !history.is_empty() {
        s.push_str("## Earlier in this conversation\n\n");
        for (q, a) in history {
            s.push_str(&format!("Q: {}\nA: {}\n\n", q.trim(), a.trim()));
        }
    }
    s.push_str(&format!("## The question\n\n{}\n\n", question.trim()));
    s.push_str(&crate::recipes::prompt_section(recipes));
    s.push('\n');
    s.push_str("## How to answer\n\nFirst one or two short plain sentences that answer the question (40 words at most; the diagram carries the detail). Then one ```shape code block: a small board that shows the answer.\n\n- Pick the form that fits the question: boxes and arrows for what depends on what, sections to group, a `timeline` for when things happened, a `table` to compare, a `mindmap` to break something down, stickies for open questions.\n- Keep it small: about 4 to 15 elements, short labels, details (`detail:`) for the rest.\n- Link every element that comes from the board to its place there: `[link: #/id]` for the top board, `[link: #/plan/id]` for an element inside canvas `plan` (the canvas path, then the id). Only ids that are on the board.\n- Start it with `title:` (a short name for the answer). Do not copy the whole board; show only what answers the question.\n- When the board does not answer it, say so in the sentences and draw what is known and what is missing (a sticky per missing piece).\n\nExample:\n\nFraud reporting is the long pole: it must ship before the open beta.\n\n```shape\ntitle: What blocks the launch\nfraud: Fraud reporting API [red, link: #/blockers/b-fraud, detail: owner rado]\nbeta: Open beta, 28 Oct [link: #/status/beta]\nfraud -> beta: must ship first\n```\n");
    s
}

/// A model's reply to [`answer_board`]: its sentences and its board (the
/// `shape` block; `diagram`, the name before, works too), or why it cannot
/// be used.
pub fn answer_parts(reply: &str) -> Result<(String, String), String> {
    let (start, fence) = ["```shape", "```diagram"]
        .iter()
        .find_map(|f| reply.find(f).map(|i| (i, *f)))
        .ok_or("the reply has no ```shape block")?;
    let body = &reply[start + fence.len()..];
    let end = body.find("```").ok_or("the ```shape block is not closed")?;
    let source = body[..end].trim_matches('\n').to_string() + "\n";
    crate::dsl::parse(&source).map_err(|e| {
        e.iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let summary = format!("{} {}", &reply[..start], &body[end + 3..])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Ok((summary, source))
}

/// A place on the board an answer points at (`[[canvas/…/id]]`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnswerRef {
    /// As written in the answer.
    pub r#ref: String,
    /// The canvas to open (ids from the top board; empty: the top board).
    pub path: Vec<String>,
    /// The element on that canvas; none when the reference is a canvas.
    pub id: Option<String>,
    /// What to call it; none when nothing on the board has that id.
    pub label: Option<String>,
}

/// The places an answer to [`question`] points at, in order (each once).
pub fn answer_refs(root: &Document, answer: &str) -> Vec<AnswerRef> {
    let mut out: Vec<AnswerRef> = Vec::new();
    let mut rest = answer;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let r = after[..end].trim().trim_matches('`').to_string();
        rest = &after[end + 2..];
        if r.is_empty() || out.iter().any(|x| x.r#ref == r) {
            continue;
        }
        let parts: Vec<String> = r.split('/').map(|p| p.trim().to_string()).collect();
        let (canvas, id) = parts.split_at(parts.len() - 1);
        let id = &id[0];
        let found = root.at(canvas).and_then(|doc| {
            if let Some(e) = doc.element(id) {
                let label = match e {
                    Element::Node(n) if !n.label.is_empty() => n.label.replace('\n', " "),
                    Element::Node(_) => id.clone(),
                    Element::Group(g) => g.display().to_string(),
                    Element::Edge(e) => e
                        .label
                        .clone()
                        .unwrap_or_else(|| format!("{} → {}", e.from, e.to)),
                };
                Some((canvas.to_vec(), Some(id.clone()), label))
            } else {
                // a whole canvas
                doc.canvas(id).map(|c| {
                    let mut p = canvas.to_vec();
                    p.push(id.clone());
                    (p, None, c.display().to_string())
                })
            }
        });
        out.push(match found {
            Some((path, id, label)) => AnswerRef {
                r#ref: r,
                path,
                id,
                label: Some(label),
            },
            None => AnswerRef {
                r#ref: r,
                path: canvas.to_vec(),
                id: Some(id.clone()),
                label: None,
            },
        });
    }
    out
}

/// The `note` in a model's reply to [`rethink`], if it has one.
pub fn reply_note(reply: &str) -> Option<String> {
    let body = crate::ops::extract_json(reply)?;
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let note = v.get("note")?.as_str()?.trim();
    (!note.is_empty()).then(|| note.to_string())
}

/// The prompt again, after a reply that could not be applied: the reply and
/// the error, to correct.
pub fn ask_again(prompt: &str, reply: &str, error: &str) -> String {
    let reply: String = reply.chars().take(4000).collect();
    format!(
        "{prompt}\n## Your previous reply could not be applied\n\n```\n{}\n```\n\nError: {error}\n\nReply again with one corrected ```json block (the whole batch).\n",
        reply.trim()
    )
}

/// The comments a prompt covers.
pub fn selected<'a>(doc: &'a Document, opts: &PromptOptions) -> Vec<&'a Comment> {
    doc.comments
        .iter()
        .filter(|c| {
            if opts.comments.is_empty() {
                c.status == Status::Open || opts.include_resolved
            } else {
                opts.comments.contains(&c.id)
            }
        })
        .collect()
}

thread_local! {
    /// The day prompts forecast gantt charts from (see [`selection_at`]).
    static TODAY: std::cell::Cell<Option<i64>> = const { std::cell::Cell::new(None) };
}

fn with_today<T>(today: Option<i64>, f: impl FnOnce() -> T) -> T {
    let before = TODAY.with(|t| t.replace(today));
    let out = f();
    TODAY.with(|t| t.set(before));
    out
}

/// [`selection`] as of `today` (days since 1970): gantt tasks also say when
/// they are forecast to end and whether they are late or behind.
pub fn selection_at(doc: &Document, ids: &[String], today: Option<i64>) -> String {
    with_today(today, || selection(doc, ids))
}

/// [`brief`] as of `today`, with gantt forecasts (see [`selection_at`]).
pub fn brief_at(doc: &Document, today: Option<i64>) -> String {
    with_today(today, || brief(doc))
}

/// A description of the diagram for pasting into a chat or giving an agent
/// context: title and counts, what the source does not spell out (gantt
/// dates, swimlane steps in order), then the DSL source with every id.
pub fn brief(doc: &Document) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# {}\n\n",
        doc.title.as_deref().unwrap_or("Diagram")
    ));
    if let Some(d) = &doc.description {
        s.push_str(&format!("{d}\n\n"));
    }
    let open = doc
        .comments
        .iter()
        .filter(|c| c.status == Status::Open)
        .count();
    s.push_str(&format!(
        "{} nodes, {} connections, {} groups, {} open comments. Flows {}.\n\n",
        doc.nodes.len(),
        doc.edges.len(),
        doc.groups.len(),
        open,
        doc.settings.direction.as_str()
    ));
    // what the source does not spell out: gantt dates and the order of
    // swimlane steps; the source follows, with every id
    let computed: Vec<String> = doc
        .groups
        .iter()
        .filter(|g| g.gantt || g.lanes || g.risks || g.entity || g.sequence)
        .map(|g| g.id.clone())
        .collect();
    if !computed.is_empty() {
        let md = selection_core(doc, &computed, true);
        let md = md.split_once("\n\n").map(|(_, r)| r).unwrap_or("");
        s.push_str(md.trim_end());
        s.push('\n');
    }
    if !s.ends_with("\n\n") {
        s.push('\n');
    }
    s.push_str(&format!(
        "## Source\n\n```shape\n{}```\n",
        dsl::print_lean(doc)
    ));
    s
}

/// A prompt asking an agent to act on review comments.
pub fn prompt(doc: &Document, opts: &PromptOptions) -> String {
    let comments = selected(doc, opts);
    let title = doc.title.as_deref().unwrap_or("Untitled diagram");
    let file = opts.file.as_deref();
    let mut s = String::new();
    match opts.target {
        Target::Diagram => {
            s.push_str(&format!(
                "# Update the diagram \"{title}\" from review comments\n\n"
            ));
            s.push_str("The diagram below is written in the `shape` language (one statement per line: `id: Label [attrs]` declares a node, `a -> b: label` connects nodes, `group id: Label { … }` groups them, `comment` / `reply` lines are review threads). Reviewers left comments on it. Address each comment listed under *Comments to address*.\n\n");
        }
        Target::Code => {
            s.push_str(&format!(
                "# Implement the changes requested in the \"{title}\" diagram review\n\n"
            ));
            s.push_str("The diagram below describes part of this codebase (its architecture, a flow or a data model). Reviewers commented on it; each comment is a requested change, a question or a concern. Make the corresponding changes in the code.\n\n");
        }
        Target::Discuss => {
            s.push_str(&format!(
                "# Answer review comments on the diagram \"{title}\"\n\n"
            ));
            s.push_str("Reviewers left comments on the diagram below. Answer each one: explain, suggest alternatives and point out risks. Do not change anything yet.\n\n");
        }
    }
    if let Some(d) = &doc.description {
        s.push_str(&format!("About the diagram: {d}\n\n"));
    }
    s.push_str("## Diagram\n\n");
    if let Some(f) = file {
        s.push_str(&format!("File: `{f}`\n\n"));
    }
    match context_ids(doc, &comments, opts.context) {
        Some(ids) => {
            let part = crate::query::subset(doc, &ids);
            let shown = part.nodes.len() + part.edges.len() + part.groups.len();
            let all = doc.nodes.len() + doc.edges.len() + doc.groups.len();
            s.push_str(&format!("The part of the board the comments are about ({shown} of its {all} elements). The rest exists too: keep its ids and do not remove or recreate it.\n\n"));
            s.push_str(&format!("```shape\n{}```\n\n", dsl::print_lean(&part)));
        }
        None => s.push_str(&format!("```shape\n{}```\n\n", dsl::print_lean(doc))),
    }
    s.push_str(&format!("## Comments to address ({})\n\n", comments.len()));
    if comments.is_empty() {
        s.push_str("_No open comments._\n");
    }
    for c in &comments {
        let n = doc.comments.iter().position(|x| x.id == c.id).unwrap() + 1;
        s.push_str(&comment_md(doc, c, n));
    }
    s.push_str("\n## How to respond\n\n");
    match (opts.target, opts.reply) {
        (Target::Diagram, Reply::Ops) => {
            s.push_str("Reply with a single ```json code block containing the operations to apply, for example:\n\n```json\n{\"ops\": [\n  {\"op\": \"add_node\", \"id\": \"cache\", \"label\": \"Redis\", \"shape\": \"cylinder\"},\n  {\"op\": \"add_edge\", \"from\": \"api\", \"to\": \"cache\", \"label\": \"reads\"},\n  {\"op\": \"resolve_comment\", \"id\": \"c1\", \"resolution\": \"Added a read-through cache\"}\n]}\n```\n\n");
            s.push_str(OPS_REFERENCE);
            s.push_str("\n\nRules:\n- Change only what the comments ask for; keep existing ids.\n- Resolve every comment you addressed with `resolve_comment` and a one-sentence `resolution`.\n- If a comment is a question or you disagree, answer with `reply` (author \"agent\") and leave it open.\n- The batch is applied atomically, so it must be valid as a whole.\n");
        }
        (Target::Diagram, Reply::Dsl) => {
            s.push_str("Reply with the complete updated diagram in a single ```shape code block. Keep existing ids. Mark every comment you addressed as resolved by adding `[resolved, resolution: what you did]` at the end of its `comment` line. For questions, add a `reply <id> by agent: …` line and leave the comment open.\n");
        }
        (Target::Code, _) => {
            s.push_str("1. Make the code changes. Follow the conventions of the codebase.\n2. For each comment, report what you did (by comment id), or why you did not change anything.\n");
            match file {
                Some(f) => s.push_str(&format!("3. If the design changed, update the diagram to match and resolve the addressed comments:\n   `shape apply {f} ops.json` (operations as below), or edit `{f}` directly and add `[resolved, resolution: …]` to the comment lines.\n\n{OPS_REFERENCE}\n")),
                None => s.push_str("3. If the design changed, also return the diagram changes as operations in a ```json {\"ops\": [...]} block, including a `resolve_comment` for each comment you addressed.\n\n"),
            }
            if file.is_none() {
                s.push_str(OPS_REFERENCE);
                s.push('\n');
            }
        }
        (Target::Discuss, _) => {
            s.push_str("Answer each comment under its id. If you want to record the answers in the diagram, also return a ```json block of `reply` operations, e.g. {\"ops\": [{\"op\": \"reply\", \"id\": \"c1\", \"author\": \"agent\", \"text\": \"…\"}]}.\n");
        }
    }
    if let Some(extra) = opts
        .instructions
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        s.push_str(&format!(
            "\n## Additional instructions\n\n{}\n",
            extra.trim()
        ));
    }
    s
}

/// The selected items (`ids`: nodes, groups, arrows) as Markdown to paste
/// into a prompt. One mindmap topic is that topic and its branches (see
/// [`crate::mindmap::to_markdown`]). Otherwise selected sections, swimlanes and
/// gantt charts bring their contents, topics bring their branches; blocks are
/// bullets under their section's heading, followed by the arrows between them
/// and their open comments.
///
/// A `#id` link reads "label (see “target”)", and what it points at that is
/// not already in the prompt follows it as a section of its own, so a prompt
/// written as a mindmap can pull in a flow or a plan from the same board.
pub fn selection(doc: &Document, ids: &[String]) -> String {
    let mut out = selection_core(doc, ids, false);
    linked(doc, ids, &mut out);
    out
}

/// The whole board (`root`, every canvas in it) as a prompt to build what
/// it describes: each board's blocks in Markdown, the arrows between them,
/// the open comments as open questions, and what to do with it. Gantt
/// forecasts are as of `today` (days since 1970) when given.
pub fn build(root: &Document, today: Option<i64>) -> String {
    let title = root.title.as_deref().unwrap_or("this board");
    let mut s = format!("# Build: {title}\n\n");
    if let Some(d) = &root.description {
        s.push_str(&format!("{d}\n\n"));
    }
    s.push_str("The board below describes what to build: the parts, how they connect, the decisions taken and the screens and flows people go through. Read all of it first, then build it. Keep the names it uses. Open comments are questions still open: where they or anything else leave it unclear, ask before you guess.\n\n");
    for (path, doc) in canvases_of(root) {
        // every top-level item: sections, blocks, mindmaps and loose boxes
        let mut top: Vec<String> = doc
            .groups
            .iter()
            .filter(|g| g.parent.is_none())
            .map(|g| g.id.clone())
            .collect();
        top.extend(
            doc.nodes
                .iter()
                .filter(|n| n.group.is_none() && n.parent.is_none())
                .map(|n| n.id.clone()),
        );
        if top.is_empty() {
            continue;
        }
        if !path.is_empty() {
            let name = canvas_at(root, &path)
                .map(|c| c.display().to_string())
                .unwrap_or_else(|| path.join("/"));
            s.push_str(&format!("## {name}\n\n"));
        }
        let md = with_today(today, || selection_core(doc, &top, false));
        // its own title is the heading above; inside a canvas, its sections
        // go one level down
        // (a lone block's title is its own: kept, as a section)
        let body = match md.strip_prefix("# ").and_then(|r| r.split_once('\n')) {
            Some((head, rest))
                if doc.title.as_deref().is_none_or(|t| t == head.trim()) || top.len() > 1 =>
            {
                rest.to_string()
            }
            Some((head, rest)) => format!("## {head}\n{rest}"),
            None => md.clone(),
        };
        for line in body.trim().lines() {
            if !path.is_empty() && line.starts_with('#') {
                s.push('#');
            }
            s.push_str(line);
            s.push('\n');
        }
        s.push('\n');
    }
    s
}

/// The canvas at `path` (ids from the top board).
fn canvas_at<'a>(root: &'a Document, path: &[String]) -> Option<&'a crate::model::Canvas> {
    let (last, outer) = path.split_last()?;
    let mut doc = root;
    for id in outer {
        doc = &doc.canvas(id)?.doc;
    }
    doc.canvas(last)
}

/// How many linked blocks one prompt pulls in, at most.
const MAX_LINKED: usize = 8;

/// Append the blocks that `#id` links in the selection point at, and the ones
/// those link to, each once.
fn linked(doc: &Document, ids: &[String], out: &mut String) {
    let mut done = covered(doc, ids);
    let mut queue = links_in(doc, &done);
    // the first block that links to each target, to say where it comes from
    let from = |t: &str, among: &[String]| {
        doc.nodes
            .iter()
            .find(|n| {
                among.contains(&n.id)
                    && n.link.as_deref().and_then(|l| l.strip_prefix('#')) == Some(t)
            })
            .map(|n| n.display().replace('\n', " "))
    };
    let mut added = 0;
    while !queue.is_empty() && added < MAX_LINKED {
        let t = queue.remove(0);
        if done.contains(&t) || (doc.node(&t).is_none() && doc.group(&t).is_none()) {
            continue;
        }
        let md = match doc.node(&t) {
            Some(n) if n.parent.is_none() && n.mindmap.is_none() => plain_md(doc, n),
            _ => selection_core(doc, std::slice::from_ref(&t), false),
        };
        out.push('\n');
        let by = from(&t, &done);
        for (k, line) in md.lines().enumerate() {
            if line.starts_with('#') {
                out.push('#');
            }
            out.push_str(line);
            if k == 0 {
                if let Some(by) = &by {
                    out.push_str(&format!(" — linked from “{by}”"));
                }
            }
            out.push('\n');
        }
        let cov = covered(doc, std::slice::from_ref(&t));
        queue.extend(
            links_in(doc, &cov)
                .into_iter()
                .filter(|l| !done.contains(l)),
        );
        done.extend(cov);
        added += 1;
    }
}

/// Everything a selection brings: groups with what is inside them, nodes,
/// and the branches of topics.
fn covered(doc: &Document, ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = ids.to_vec();
    loop {
        let more: Vec<String> = doc
            .groups
            .iter()
            .filter(|g| g.parent.as_ref().is_some_and(|p| out.contains(p)))
            .map(|g| g.id.clone())
            .chain(
                doc.nodes
                    .iter()
                    .filter(|n| {
                        n.group.as_ref().is_some_and(|g| out.contains(g))
                            || n.parent.as_ref().is_some_and(|p| out.contains(p))
                    })
                    .map(|n| n.id.clone()),
            )
            .filter(|id| !out.contains(id))
            .collect();
        if more.is_empty() {
            return out;
        }
        out.extend(more);
    }
}

/// The targets of `#id` links on these nodes, in document order.
fn links_in(doc: &Document, ids: &[String]) -> Vec<String> {
    doc.nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .filter_map(|n| n.link.as_deref()?.strip_prefix('#'))
        .map(str::to_string)
        .collect()
}

/// A single block: its label, detail, where it is and its arrows.
fn plain_md(doc: &Document, n: &Node) -> String {
    let label = |id: &str| {
        doc.node(id)
            .map(|n| n.display().replace('\n', " "))
            .or_else(|| doc.group(id).map(|g| g.display().to_string()))
            .unwrap_or_else(|| id.to_string())
    };
    let mut s = format!("# {}\n", n.display().replace('\n', " "));
    if let Some(d) = &n.detail {
        s.push_str(&format!("\n{d}\n"));
    }
    if let Some(g) = n.group.as_deref().and_then(|g| doc.group(g)) {
        s.push_str(&format!("\nIn “{}”.\n", g.display()));
    }
    let mut arrows = String::new();
    for e in &doc.edges {
        let l = e
            .label
            .as_deref()
            .map(|l| format!(" ({l})"))
            .unwrap_or_default();
        if e.from == n.id {
            arrows.push_str(&format!("- → {}{l}\n", label(&e.to)));
        } else if e.to == n.id {
            arrows.push_str(&format!("- ← {}{l}\n", label(&e.from)));
        }
    }
    if !arrows.is_empty() {
        s.push_str(&format!("\n{arrows}"));
    }
    s
}

/// A label for a prompt: a web link stays a Markdown link, a `#id` link says
/// what it points at (which the prompt includes).
pub(crate) fn link_md(doc: &Document, label: &str, link: Option<&str>) -> String {
    match link {
        Some(l) if l.starts_with('#') => {
            let t = &l[1..];
            match doc
                .node(t)
                .map(|n| n.display().replace('\n', " "))
                .or_else(|| doc.group(t).map(|g| g.display().to_string()))
            {
                Some(name) => format!("{label} (see “{name}”)"),
                None => label.to_string(),
            }
        }
        Some(l) => match crate::model::board_link(l) {
            Some((path, Some(id))) => format!("{label} (see `{id}` on {path})"),
            Some((path, None)) => format!("{label} (see {path})"),
            None => format!("[{label}]({l})"),
        },
        None => label.to_string(),
    }
}

/// [`selection`] without linked blocks. `titled` always heads it with the
/// board's title (otherwise one section selected is its own title).
fn selection_core(doc: &Document, ids: &[String], titled: bool) -> String {
    if let [one] = ids {
        if doc
            .node(one)
            .is_some_and(|n| n.parent.is_some() || n.mindmap.is_some())
        {
            return crate::mindmap::to_markdown(doc, one);
        }
    }
    // what is in: selected groups with everything inside them, selected nodes
    let mut groups: Vec<&str> = doc
        .groups
        .iter()
        .filter(|g| ids.contains(&g.id))
        .map(|g| g.id.as_str())
        .collect();
    loop {
        let more: Vec<&str> = doc
            .groups
            .iter()
            .filter(|g| !groups.contains(&g.id.as_str()))
            .filter(|g| g.parent.as_deref().is_some_and(|p| groups.contains(&p)))
            .map(|g| g.id.as_str())
            .collect();
        if more.is_empty() {
            break;
        }
        groups.extend(more);
    }
    let nodes: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id) || n.group.as_deref().is_some_and(|g| groups.contains(&g)))
        .collect();
    let has = |id: &str| nodes.iter().any(|n| n.id == id);
    let mut out = String::new();
    // one section selected: its title is the title
    let top: Vec<&Group> = doc
        .groups
        .iter()
        .filter(|g| groups.contains(&g.id.as_str()))
        .filter(|g| !g.parent.as_deref().is_some_and(|p| groups.contains(&p)))
        .collect();
    let solo = !titled
        && top.len() == 1
        && nodes
            .iter()
            .all(|n| n.group.is_some() && groups.contains(&n.group.as_deref().unwrap()));
    let mut seen: Vec<String> = Vec::new();
    // steps of swimlanes: their arrows are part of the steps
    let mut flowed: Vec<String> = Vec::new();
    // blocks not inside an included group
    let loose: Vec<&Node> = nodes
        .iter()
        .copied()
        .filter(|n| !n.group.as_deref().is_some_and(|g| groups.contains(&g)))
        .filter(|n| !n.parent.as_deref().is_some_and(|p| has(p)))
        .collect();
    let block = |n: &Node, out: &mut String, seen: &mut Vec<String>| {
        seen.push(n.id.clone());
        out.push_str(&format!("- {}\n", item(doc, n)));
        crate::mindmap::bullets(doc, &n.id, 1, out, seen);
    };
    for n in &loose {
        block(n, &mut out, &mut seen);
    }
    fn section<'a>(
        doc: &'a Document,
        g: &Group,
        level: usize,
        groups: &[&str],
        nodes: &[&'a Node],
        out: &mut String,
        seen: &mut Vec<String>,
        flowed: &mut Vec<String>,
        block: &dyn Fn(&Node, &mut String, &mut Vec<String>),
    ) {
        if !out.is_empty() {
            out.push('\n');
        }
        let kind = if g.sequence {
            " (sequence diagram)".to_string()
        } else if let Some(st) = &g.adr {
            format!(" (decision record, {st})")
        } else if g.entity {
            format!(" (database table `{}`)", g.id)
        } else if g.forecast {
            format!(" (forecast in {}: now {}; each launch adds this much a month, reached after `ramp` months, from its gantt milestone's date)", g.unit.as_deref().unwrap_or("units"), crate::forecast::format_amount(g.now.unwrap_or(0.0)))
        } else if g.schema {
            match &g.file {
                Some(f) => {
                    format!(" (database schema, from `{f}`: change the file, not these tables)")
                }
                None => " (database schema)".to_string(),
            }
        } else if g.table {
            " (table)".to_string()
        } else if g.timeline {
            " (timeline)".to_string()
        } else if g.cycle {
            " (cycle: each step leads to the next, the last back to the first)".to_string()
        } else if let Some(d) = g.screen {
            format!(
                " (wireframe screen, {}: its components top to bottom)",
                d.as_str()
            )
        } else if g.todo {
            let (done, all) = crate::todo::progress(doc, &g.id);
            format!(" (to-do list, {done} of {all} done)")
        } else if let Some(c) = g.chart {
            let mut k = format!(" ({} chart", c.as_str());
            if g.stacked {
                k.push_str(", stacked");
            }
            if g.horizontal {
                k.push_str(", horizontal");
            }
            if let Some(u) = &g.unit {
                k.push_str(&format!(", values in {u}"));
            }
            k.push(')');
            k
        } else if g.risks {
            let n = crate::risks::levels(g);
            format!(" (risk matrix, probability × impact, {n}×{n})")
        } else if g.gantt {
            match g.start.as_deref() {
                Some(s) => format!(" (gantt chart from {s})"),
                None => " (gantt chart)".into(),
            }
        } else if g.kanban {
            " (kanban board)".into()
        } else if g.lanes {
            " (swimlanes)".into()
        } else if g
            .parent
            .as_deref()
            .and_then(|p| doc.group(p))
            .is_some_and(|p| p.lanes)
        {
            " (lane)".into()
        } else {
            String::new()
        };
        let kind = if plain() { String::new() } else { kind };
        out.push_str(&format!(
            "{} {}{kind}\n",
            "#".repeat(level.min(6)),
            g.display()
        ));
        let inside: Vec<&&Node> = nodes
            .iter()
            .filter(|n| n.group.as_deref() == Some(&g.id))
            .collect();
        if !inside.is_empty() {
            out.push('\n');
        }
        if g.gantt {
            let s = crate::gantt::schedule(doc, &g.id);
            let forecast = TODAY
                .with(|t| t.get())
                .map(|t| crate::gantt::track(doc, &g.id, t))
                .unwrap_or_default();
            for (i, start, days) in s.tasks {
                let n = &doc.nodes[i];
                let when = match s.origin {
                    Some(o) => {
                        let from = o + start.floor() as i64;
                        let to = from + (days.ceil() as i64 - 1).max(0);
                        if to > from {
                            format!(
                                "{} → {}, ",
                                crate::gantt::format_date(from),
                                crate::gantt::format_date(to)
                            )
                        } else {
                            format!("{}, ", crate::gantt::format_date(from))
                        }
                    }
                    None => String::new(),
                };
                let after: Vec<&str> = n
                    .after
                    .iter()
                    .map(|a| doc.node(a).map(|t| t.display()).unwrap_or(a))
                    .collect();
                let after = if after.is_empty() {
                    String::new()
                } else {
                    format!(", after {}", after.join(", "))
                };
                let mut more = String::new();
                match n.done {
                    Some(d) if d >= 100.0 => more.push_str(", done"),
                    Some(d) => more.push_str(&format!(", {}% done", d.round())),
                    None => {}
                }
                if let Some(o) = &n.owner {
                    more.push_str(&format!(", owner {o}"));
                }
                let length = if n.milestone {
                    "milestone".to_string()
                } else {
                    crate::gantt::format_days(days)
                };
                // as of today: the forecast, when it is not the plan
                if let Some(t) = forecast.iter().find(|t| t.id == n.id) {
                    if t.slip > 0 {
                        let what = if n.milestone {
                            "forecast"
                        } else {
                            "forecast end"
                        };
                        more.push_str(&format!(
                            ", {what} {} (+{}d, {})",
                            t.forecast_end, t.slip, t.status
                        ));
                    } else if matches!(t.status, "late" | "behind") {
                        more.push_str(&format!(", {}", t.status));
                    }
                }
                seen.push(n.id.clone());
                out.push_str(&format!(
                    "- {}: {when}{length}{after}{more}\n",
                    item(doc, n)
                ));
            }
        } else if g.chart.is_some() {
            chart_table(doc, g, out, seen);
        } else if g.table {
            data_table(doc, g, out, seen);
        } else if g.todo {
            // a checklist, sub-items indented
            for x in crate::todo::items_of(doc, &g.id) {
                seen.push(x.id.clone());
                let mut line = format!(
                    "{}- [{}] {}",
                    "  ".repeat(x.level as usize),
                    if crate::todo::is_done(x) { "x" } else { " " },
                    x.display().replace('\n', " ")
                );
                let mut more = Vec::new();
                if let Some(o) = &x.owner {
                    more.push(o.clone());
                }
                if let Some(d) = &x.date {
                    more.push(format!("due {d}"));
                }
                if !more.is_empty() {
                    line.push_str(&format!(" ({})", more.join(", ")));
                }
                if let Some(d) = shown(&x.detail) {
                    line.push_str(&format!(" — {}", d.replace('\n', " ")));
                }
                out.push_str(&line);
                out.push('\n');
            }
        } else if g.screen.is_some() {
            // a wireframe: each component with what it is, top to bottom
            for x in doc
                .nodes
                .iter()
                .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
            {
                seen.push(x.id.clone());
                let kind = x.widget.unwrap_or_default().as_str();
                let mut line = format!("- {kind}");
                if !x.label.is_empty() {
                    line.push_str(&format!(": {}", x.label.replace('\n', " ")));
                }
                if let Some(c) = &x.color {
                    line.push_str(&format!(" ({c})"));
                }
                if let Some(d) = shown(&x.detail) {
                    line.push_str(&format!(" — {}", d.replace('\n', " ")));
                }
                if !plain() {
                    line.push_str(&format!(" `{}`", x.id));
                }
                out.push_str(&line);
                out.push('\n');
            }
        } else if g.timeline {
            // in time order, important ones marked
            let mut list = crate::timeline::events_of(doc, &g.id);
            list.sort_by_key(|x| {
                x.date
                    .as_deref()
                    .and_then(crate::timeline::parse)
                    .map_or(i64::MAX, |d| d.0)
            });
            for x in list {
                seen.push(x.id.clone());
                let label = link_md(doc, &x.display().replace('\n', " "), x.link.as_deref());
                let when = x.date.as_deref().unwrap_or("?");
                let star = if x.important { " (important)" } else { "" };
                let mut line = format!("- {when}: {label}{star}");
                if let Some(d) = shown(&x.detail) {
                    line.push_str(&format!(" — {}", d.replace('\n', " ")));
                }
                out.push_str(&line);
                out.push('\n');
            }
        } else if g.entity {
            column_table(doc, g, out, seen);
        } else if g.risks {
            // worst first
            let n = crate::risks::levels(g);
            let mut list = crate::risks::risks_of(doc, &g.id);
            let score = |x: &Node| {
                crate::risks::rating(x, n).map_or(0, |(p, i)| u16::from(p) * u16::from(i))
            };
            list.sort_by_key(|x| std::cmp::Reverse(score(x)));
            for x in list {
                seen.push(x.id.clone());
                let label = link_md(doc, &x.display().replace('\n', " "), x.link.as_deref());
                let mut line = format!("- {label}: {}", crate::risks::summary(x, n).to_lowercase());
                if let Some(o) = &x.owner {
                    line.push_str(&format!(", owner {o}"));
                }
                if let Some(d) = shown(&x.detail) {
                    line.push_str(&format!(" — {}", d.replace('\n', " ")));
                }
                out.push_str(&line);
                out.push('\n');
            }
        } else if g.sequence {
            let people: Vec<String> = crate::sequence::participants_of(doc, &g.id)
                .iter()
                .map(|n| {
                    seen.push(n.id.clone());
                    // messages are the steps below, not connections
                    flowed.push(n.id.clone());
                    n.display().replace('\n', " ")
                })
                .collect();
            out.push_str(&format!("Participants: {}.\n\n", people.join(", ")));
            for (k, e) in crate::sequence::messages_of(doc, &g.id).iter().enumerate() {
                let name = |id: &str| {
                    doc.node(id)
                        .map_or(id.to_string(), |n| n.display().replace('\n', " "))
                };
                let what = e
                    .label
                    .as_deref()
                    .map(|l| format!(": {l}"))
                    .unwrap_or_default();
                let reply = if e.line == Line::Dashed {
                    " (reply)"
                } else {
                    ""
                };
                out.push_str(&format!(
                    "{}. {} → {}{what}{reply}\n",
                    k + 1,
                    name(&e.from),
                    name(&e.to)
                ));
            }
        } else if g.adr.is_some() {
            for f in crate::adr::fields_of(doc, &g.id) {
                seen.push(f.id.clone());
                let text = f.detail.as_deref().unwrap_or("—").replace('\n', " ");
                out.push_str(&format!("**{}:** {text}\n", f.display()));
            }
        } else if g.kanban {
            kanban_md(doc, g, out, seen);
        } else if g.lanes {
            out.push('\n');
            steps(doc, g, out, seen, flowed);
        } else {
            for n in inside {
                if !n
                    .parent
                    .as_deref()
                    .is_some_and(|p| nodes.iter().any(|m| m.id == p))
                {
                    block(n, out, seen);
                }
            }
        }
        for sub in doc
            .groups
            .iter()
            .filter(|s| s.parent.as_deref() == Some(&g.id))
        {
            if groups.contains(&sub.id.as_str()) && !g.lanes {
                section(doc, sub, level + 1, groups, nodes, out, seen, flowed, block);
            }
        }
    }
    for g in &top {
        section(
            doc,
            g,
            if solo { 1 } else { 2 },
            &groups,
            &nodes,
            &mut out,
            &mut seen,
            &mut flowed,
            &block,
        );
    }
    // everything listed, topics included
    let listed = |id: &str| seen.iter().any(|s| s == id) || groups.contains(&id);
    let label = |id: &str| {
        doc.node(id)
            .map(|n| n.display().replace('\n', " "))
            .or_else(|| doc.group(id).map(|g| g.display().to_string()))
            .unwrap_or_else(|| id.to_string())
    };
    let arrows: Vec<&Edge> = doc
        .edges
        .iter()
        .filter(|e| ids.contains(&e.id) || (listed(&e.from) && listed(&e.to)))
        .filter(|e| !(flowed.contains(&e.from) && flowed.contains(&e.to)))
        .filter(|e| {
            !doc.node(&e.from).is_some_and(|n| {
                n.group
                    .as_deref()
                    .and_then(|g| doc.group(g))
                    .is_some_and(|g| g.gantt)
                    && doc.node(&e.to).is_some_and(|m| m.group == n.group)
            })
        })
        .collect();
    if !arrows.is_empty() {
        out.push_str("\n## Connections\n\n");
        for e in arrows {
            let l = e
                .label
                .as_deref()
                .map(|l| format!(": {l}"))
                .unwrap_or_default();
            out.push_str(&format!("- {} → {}{l}\n", label(&e.from), label(&e.to)));
        }
    }
    let comments: Vec<&Comment> = doc
        .comments
        .iter()
        .filter(|c| c.status == Status::Open)
        .filter(|c| {
            c.on.as_deref()
                .is_some_and(|on| listed(on) || ids.iter().any(|i| i == on))
        })
        .collect();
    if !comments.is_empty() {
        out.push_str("\n## Comments\n\n");
        for c in comments {
            out.push_str(&format!(
                "- On {}: {}\n",
                label(c.on.as_deref().unwrap()),
                c.text.replace('\n', " ")
            ));
            for r in &c.replies {
                out.push_str(&format!("  - {}\n", r.text.replace('\n', " ")));
            }
        }
    }
    let out = out.trim_start_matches('\n').to_string();
    if solo {
        out
    } else {
        let title = doc.title.as_deref().unwrap_or("Selection");
        format!("# {title}\n\n{out}")
    }
}

/// A database table as a Markdown table of its columns.
fn column_table(doc: &Document, g: &Group, out: &mut String, seen: &mut Vec<String>) {
    out.push_str(
        "| column | type | key | null | references | default |\n|---|---|---|---|---|---|\n",
    );
    for c in crate::entities::columns_of(doc, &g.id) {
        seen.push(c.id.clone());
        let col = c.column.clone().unwrap_or_default();
        let key = [(col.pk, "PK"), (col.unique, "unique")]
            .iter()
            .filter(|(on, _)| *on)
            .map(|(_, k)| *k)
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "| {} | {} | {key} | {} | {} | {} |\n",
            c.display(),
            col.ty.replace('|', "\\|"),
            if col.null { "yes" } else { "no" },
            col.fk.as_deref().unwrap_or(""),
            col.default.as_deref().unwrap_or("")
        ));
    }
}

/// A table as a Markdown table.
fn data_table(doc: &Document, g: &Group, out: &mut String, seen: &mut Vec<String>) {
    let rows = crate::tables::rows_of(doc, &g.id);
    let n = rows
        .iter()
        .map(|r| r.cells.len() + 1)
        .max()
        .unwrap_or(1)
        .max(g.columns.len())
        .max(1);
    let cell = |s: &str| s.replace('|', "\\|").replace('\n', "<br>");
    let head: Vec<String> = (0..n)
        .map(|k| cell(g.columns.get(k).map(|s| s.as_str()).unwrap_or("")))
        .collect();
    out.push_str(&format!(
        "| {} |\n|{}\n",
        head.join(" | "),
        "---|".repeat(n)
    ));
    for r in rows {
        seen.push(r.id.clone());
        let mut cells = vec![link_md(doc, &cell(r.display()), r.link.as_deref())];
        cells.extend((1..n).map(|k| cell(r.cells.get(k - 1).map(|s| s.as_str()).unwrap_or(""))));
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
}

/// A chart as a Markdown table: the header, one row per row (a pie's also
/// with each slice's share).
fn chart_table(doc: &Document, g: &Group, out: &mut String, seen: &mut Vec<String>) {
    let rows = crate::charts::rows_of(doc, &g.id);
    let n = crate::charts::series_count(doc, g);
    let pie = matches!(g.chart, Some(ChartKind::Pie | ChartKind::Donut));
    let total: f64 = rows
        .iter()
        .map(|r| r.values.first().copied().flatten().unwrap_or(0.0).max(0.0))
        .sum();
    let mut head = vec![g.axis.clone().unwrap_or_default()];
    head.extend((0..n).map(|k| {
        if g.series.is_empty() && n == 1 {
            "value".to_string()
        } else {
            crate::charts::series_name(g, k)
        }
    }));
    if pie {
        head.push("share".into());
    }
    let cell = |s: &str| s.replace('|', "/").replace('\n', " ");
    out.push_str(&format!(
        "| {} |\n|{}\n",
        head.iter().map(|h| cell(h)).collect::<Vec<_>>().join(" | "),
        "---|".repeat(head.len())
    ));
    for r in rows {
        seen.push(r.id.clone());
        let mut cells = vec![link_md(doc, &cell(r.display()), r.link.as_deref())];
        cells.extend((0..n).map(|k| dsl::fmt_value(r.values.get(k).copied().flatten())));
        if pie {
            let v = r.values.first().copied().flatten().unwrap_or(0.0).max(0.0);
            cells.push(if total > 0.0 {
                format!("{}%", (v / total * 100.0).round())
            } else {
                "-".into()
            });
        }
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
    }
}

/// A kanban board: each column in bold with its cards, top to bottom.
fn kanban_md(doc: &Document, g: &Group, out: &mut String, seen: &mut Vec<String>) {
    let cards = |id: &str| -> Vec<&Node> {
        doc.nodes
            .iter()
            .filter(|n| n.group.as_deref() == Some(id))
            .collect()
    };
    let mut cols: Vec<(String, Vec<&Node>)> = doc
        .groups
        .iter()
        .filter(|c| c.parent.as_deref() == Some(&g.id))
        .map(|c| (c.display().to_string(), cards(&c.id)))
        .collect();
    let loose = cards(&g.id);
    if !loose.is_empty() {
        cols.push(("No column".into(), loose));
    }
    for (name, list) in cols {
        out.push_str(&format!("\n**{name}** ({})\n", list.len()));
        for n in list {
            seen.push(n.id.clone());
            out.push_str(&format!("- {}\n", item(doc, n)));
        }
    }
}

/// Swimlanes as numbered steps in the order the flow runs (as laid out), each
/// with its lane; where a step does not simply lead to the next one, its
/// arrows say which step comes next (`yes → 4. Confirm order`).
fn steps(
    doc: &Document,
    g: &Group,
    out: &mut String,
    seen: &mut Vec<String>,
    flowed: &mut Vec<String>,
) {
    let lanes: Vec<&Group> = doc
        .groups
        .iter()
        .filter(|l| l.parent.as_deref() == Some(&g.id))
        .collect();
    let lane_of = |n: &Node| lanes.iter().position(|l| n.group.as_deref() == Some(&l.id));
    let mut list: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| n.parent.is_none() && lane_of(n).is_some())
        .collect();
    let lay = crate::layout::layout(doc);
    let along = |n: &Node| {
        let Some(b) = lay.nodes.iter().find(|b| b.id == n.id) else {
            return 0.0;
        };
        let (cx, cy) = (b.rect.x + b.rect.w / 2.0, b.rect.y + b.rect.h / 2.0);
        match doc.settings.direction {
            Direction::Right => cx,
            Direction::Left => -cx,
            Direction::Down => cy,
            Direction::Up => -cy,
        }
        .round()
    };
    list.sort_by(|a, b| {
        along(a)
            .total_cmp(&along(b))
            .then(lane_of(a).cmp(&lane_of(b)))
    });
    if !lanes.is_empty() {
        let names: Vec<&str> = lanes.iter().map(|l| l.display()).collect();
        out.push_str(&format!("Lanes: {}.\n\n", names.join(", ")));
    }
    let at = |id: &str| list.iter().position(|n| n.id == id);
    for (k, n) in list.iter().enumerate() {
        seen.push(n.id.clone());
        flowed.push(n.id.clone());
        let lane = lane_of(n).map(|l| lanes[l].display()).unwrap_or_default();
        let kind = if n.shape == Shape::Diamond {
            ", decision"
        } else {
            ""
        };
        out.push_str(&format!("{}. {} — {lane}{kind}\n", k + 1, item(doc, n)));
        let outs: Vec<&Edge> = doc
            .edges
            .iter()
            .filter(|e| e.from == n.id && at(&e.to).is_some())
            .collect();
        let simple = outs.len() == 1 && outs[0].label.is_none() && at(&outs[0].to) == Some(k + 1);
        if simple {
            continue;
        }
        for e in outs {
            let j = at(&e.to).unwrap();
            let l = e
                .label
                .as_deref()
                .map(|l| format!("{} ", l.replace('\n', " ")))
                .unwrap_or_default();
            out.push_str(&format!(
                "   - {l}→ {}. {}\n",
                j + 1,
                list[j].display().replace('\n', " ")
            ));
        }
    }
}

thread_local! {
    /// Writing for people (Copy as text): what the board shows, without the
    /// details, the ids and the notes for a model.
    static PLAIN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn plain() -> bool {
    PLAIN.with(|p| p.get())
}

/// A detail, unless writing for people.
fn shown(d: &Option<String>) -> Option<&String> {
    d.as_ref().filter(|_| !plain())
}

/// The selection as text for people (Copy as text): the items as the board
/// shows them, in Markdown (a to-do list as a checklist, a map as indented
/// bullets, a table as a table), without the details behind them, the ids
/// or the notes for a model. One block comes without its title.
pub fn plain_text(doc: &Document, ids: &[String], today: Option<i64>) -> String {
    // a mindmap (or a branch of one): its topics as indented bullets
    if let [one] = ids {
        if doc
            .node(one)
            .is_some_and(|n| n.parent.is_some() || n.mindmap.is_some())
        {
            fn walk(doc: &Document, id: &str, depth: usize, out: &mut String) {
                for c in doc.children(id) {
                    out.push_str(&format!(
                        "{}- {}\n",
                        "  ".repeat(depth),
                        c.display().replace('\n', " ")
                    ));
                    walk(doc, &c.id, depth + 1, out);
                }
            }
            let mut out = format!(
                "{}\n\n",
                doc.node(one)
                    .map(|n| n.display().replace('\n', " "))
                    .unwrap_or_default()
            );
            walk(doc, one, 0, &mut out);
            return out.trim_end().to_string() + "\n";
        }
    }
    let before = PLAIN.with(|p| p.replace(true));
    let mut out = with_today(today, || selection_core(doc, ids, false));
    PLAIN.with(|p| p.set(before));
    // one block: its items, without its heading
    let single = ids.len() == 1
        && doc.group(&ids[0]).is_some_and(|g| {
            doc.groups
                .iter()
                .all(|c| c.parent.as_deref() != Some(g.id.as_str()))
        });
    if single {
        if let Some(rest) = out
            .strip_prefix('#')
            .and_then(|r| r.split_once('\n'))
            .map(|(_, r)| r)
        {
            out = rest.trim_start_matches('\n').to_string();
        }
    }
    out.trim_end().to_string() + "\n"
}

/// A block as one Markdown line: label, then its detail and link.
fn item(doc: &Document, n: &Node) -> String {
    let mut s = link_md(doc, &n.display().replace('\n', " "), n.link.as_deref());
    if let Some(d) = shown(&n.detail) {
        s.push_str(&format!(" — {}", d.replace('\n', " ")));
    }
    s
}

#[cfg(test)]
mod plain_tests {
    #[test]
    fn copy_as_text_is_what_the_board_shows() {
        let d = crate::dsl::parse("todo plan: Rollout {\n  [ ] Staging [detail: Before production, every step]\n    [ ] Create the accounts (Ops) [detail: The old ones are gone]\n    [x] Rotate the keys [owner: ana]\n  [ ] Production\n    [ ] Same again [detail: After staging]\n}\nmindmap m: Ideas {\n  One [detail: why one]\n    One a\n  Two\n}\n").unwrap();
        let t = super::plain_text(&d, &["plan".into()], None);
        assert_eq!(t, "- [ ] Staging\n  - [ ] Create the accounts (Ops)\n  - [x] Rotate the keys (ana)\n- [ ] Production\n  - [ ] Same again\n");
        // the prompt keeps the details
        assert!(super::selection(&d, &["plan".into()]).contains("Before production, every step"));
        let m = super::plain_text(&d, &["m".into()], None);
        assert_eq!(m, "Ideas\n\n- One\n  - One a\n- Two\n");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_whole_board_as_a_prompt_to_build_it() {
        let d = crate::dsl::parse("title: Shop\ndescription: A small shop\n\ngroup api: API {\n  orders: Orders [detail: Rust]\n}\nweb: Storefront\nweb -> orders: POST\ncomment on orders: Which database?\npay: Pay [canvas: pay]\n\ncanvas pay: Payments {\n  screen card: Card [mobile] {\n    input number: Card number\n    button go: Pay [blue]\n  }\n}\n").unwrap();
        let p = build(&d, None);
        assert!(
            p.starts_with(
                "# Build: Shop\n\nA small shop\n\nThe board below describes what to build"
            ),
            "{p}"
        );
        assert!(p.contains("## API\n\n- Orders — Rust"), "{p}");
        assert!(p.contains("- Storefront → Orders: POST"), "{p}");
        assert!(p.contains("Which database?"), "{p}");
        // a canvas under its own heading, its blocks one level down; screens by component
        assert!(
            p.contains("## Payments\n\n### Card (wireframe screen, mobile"),
            "{p}"
        );
        assert!(
            p.contains("- input: Card number `card.number`\n- button: Pay (blue) `card.go`"),
            "{p}"
        );
        assert_eq!(p.matches("# Shop").count(), 0, "no repeated title");
    }

    use super::*;

    fn doc() -> Document {
        dsl::parse("title: Shop\napi: Orders API\napi -> db\ncomment on api by deyan: Split payments out?\nreply c1 by ana: Maybe later\ncomment on api->db: Use a replica [resolved]").unwrap()
    }

    #[test]
    fn prompt_lists_open_comments_and_instructions() {
        let p = prompt(&doc(), &PromptOptions::default());
        assert!(
            p.contains("**c1** on node `api` (\"Orders API\") — @deyan"),
            "{p}"
        );
        assert!(p.contains("@ana: Maybe later"));
        assert!(!p.contains("**c2**"));
        assert!(p.contains("resolve_comment"));
        let p = prompt(
            &doc(),
            &PromptOptions {
                include_resolved: true,
                target: Target::Code,
                file: Some("docs/shop.shape".into()),
                ..Default::default()
            },
        );
        assert!(p.contains("**c2** on edge `api` → `db` (resolved)"), "{p}");
        assert!(p.contains("shape apply docs/shop.shape"));
    }

    #[test]
    fn prompts_show_the_part_of_a_big_board_the_comments_are_about() {
        let mut src = String::from(
            "title: Big\ngroup back: Backend {\n  api: Orders API\n  db: Postgres\n}\napi -> db\n",
        );
        for i in 0..70 {
            src.push_str(&format!("n{i}: Box {i}\n"));
        }
        src.push_str("comment on db by ana: Which version?\n");
        let d = dsl::parse(&src).unwrap();
        let auto = prompt(&d, &PromptOptions::default());
        assert!(
            auto.contains("The part of the board the comments are about (4 of its 74 elements)"),
            "{auto}"
        );
        assert!(
            auto.contains("group back: Backend {\n  api: Orders API\n  db: Postgres\n}")
                && !auto.contains("n42"),
            "{auto}"
        );
        let all = prompt(
            &d,
            &PromptOptions {
                context: Context::All,
                ..Default::default()
            },
        );
        assert!(all.contains("n42: Box 42") && !all.contains("The part of the board"));
        // a small board goes in whole
        assert!(!prompt(&doc(), &PromptOptions::default()).contains("The part of the board"));
    }

    #[test]
    fn selections_become_prompts() {
        let d = dsl::parse("title: Shop\ngroup back: Backend {\n  api: Orders API\n  db: Database [detail: Postgres]\n}\nweb: Web app\nweb -> api: places orders\napi -> db\ncomment on db: Which version?\ngantt g: Plan [start: 2026-10-12] {\n  Design [5d]\n  Build [1w, after: design]\n}\nmindmap m: Goals {\n  Fast\n    Under 1s\n}").unwrap();
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            selection(&d, &ids(&["web", "back"])),
            "# Shop\n\n- Web app\n\n## Backend\n\n- Orders API\n- Database — Postgres\n\n## Connections\n\n- Web app → Orders API: places orders\n- Orders API → Database\n\n## Comments\n\n- On Database: Which version?\n"
        );
        assert_eq!(
            selection(&d, &ids(&["back"])),
            "# Backend\n\n- Orders API\n- Database — Postgres\n\n## Connections\n\n- Orders API → Database\n\n## Comments\n\n- On Database: Which version?\n"
        );
        assert_eq!(
            selection(&d, &ids(&["g"])),
            "# Plan (gantt chart from 2026-10-12)\n\n- Design: 2026-10-12 → 2026-10-16, 5d\n- Build: 2026-10-17 → 2026-10-23, 1w, after Design\n"
        );
        assert_eq!(
            selection(&d, &ids(&["m"])),
            crate::mindmap::to_markdown(&d, "m")
        );
        assert_eq!(
            selection(&d, &ids(&["m", "web"])),
            "# Shop\n\n- Web app\n- Goals\n  - Fast\n    - Under 1s\n"
        );
    }

    #[test]
    fn brief_describes_everything() {
        let b = brief(&doc());
        assert!(b.starts_with("# Shop"));
        assert!(b.contains("```shape\n"));
        assert!(
            b.contains("comment c1 on api by deyan: Split payments out?"),
            "{b}"
        );
        // gantt charts come with their dates
        let d =
            dsl::parse("title: P\ngantt g: Plan [start: 2026-10-12] {\n  Design [5d]\n}").unwrap();
        assert!(
            brief(&d).contains(
                "## Plan (gantt chart from 2026-10-12)\n\n- Design: 2026-10-12 → 2026-10-16, 5d\n"
            ),
            "{}",
            brief(&d)
        );
    }

    #[test]
    fn links_bring_what_they_point_at() {
        let d = dsl::parse("title: Board\nmindmap task: Task {\n  Context\n    The flow [link: #flow]\n    Docs [link: https://x.dev]\n}\ngroup flow: Flow {\n  a: Start [link: #note]\n  b: End\n}\nnote: Ask ops [detail: before launch]\na -> b\nnote -> b: informs").unwrap();
        let p = selection(&d, &["task".to_string()]);
        assert_eq!(
            p,
            "# Task\n\n## Context\n\n- The flow (see “Flow”)\n- [Docs](https://x.dev)\n\n## Flow — linked from “The flow”\n\n- Start (see “Ask ops”)\n- End\n\n### Connections\n\n- Start → End\n\n## Ask ops — linked from “Start”\n\nbefore launch\n\n- → End (informs)\n"
        );
        // already in the prompt: not repeated
        let p = selection(
            &d,
            &["task".to_string(), "flow".to_string(), "note".to_string()],
        );
        assert!(!p.contains("linked from"), "{p}");
    }

    #[test]
    fn swimlanes_are_numbered_steps() {
        let d = dsl::parse("title: T\nlanes o: Orders {\n  lane c: Customer {\n    place: Place order\n    fix: Fix details\n  }\n  lane s: Sales {\n    ok: Credit OK? [diamond]\n    confirm: Confirm\n  }\n}\nplace -> ok\nok -> confirm: yes\nok -> fix: no\nfix -> ok").unwrap();
        let p = selection(&d, &["o".to_string()]);
        assert!(p.starts_with("# Orders (swimlanes)\n\nLanes: Customer, Sales.\n\n1. Place order — Customer\n2. Credit OK? — Sales, decision\n"), "{p}");
        assert!(p.contains("   - yes → "), "{p}");
        assert!(p.contains("   - no → "), "{p}");
        assert!(!p.contains("Connections"), "{p}");
    }

    #[test]
    fn ask_prompts_show_the_selection_and_ask_for_ops() {
        let d = dsl::parse("title: Shop\ngroup back: Backend {\n  api: Orders API\n  db: Postgres\n}\nweb: Web\nmail: Mail\nweb -> api\napi -> db\n").unwrap();
        let p = ask(&d, &["db".to_string()], "Add a read replica", &[]);
        assert!(
            p.contains("Selected: `db`.") && p.contains("api -> db") && !p.contains("mail: Mail"),
            "{p}"
        );
        assert!(
            p.contains("## The request\n\nAdd a read replica") && p.contains("{\"op\":\"merge\""),
            "{p}"
        );
        let whole = ask(&d, &[], "Tidy", &[]);
        assert!(whole.contains("mail: Mail"));
        let again = ask_again(&p, "{oops", "invalid JSON");
        assert!(again.starts_with(&p) && again.contains("Error: invalid JSON"));
    }
}
