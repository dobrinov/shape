<img src="web/favicon.svg" width="64" alt="Shape logo">

# Shape

**From a messy idea to a plan everyone gets.** Shape is one board for the whole project: think in mindmaps, plan on kanban boards and timelines, sketch the screens, write down the decisions, then present it. The AI you already pay for (Claude, ChatGPT or Gemini) sorts what you throw at it, keeps the board up to date and answers questions with a drawing.

**It is also a way to write prompts.** For a complex build, the best prompt is a picture of it: lay out the system, the screens, the data model and the decisions, then copy the whole board (or a selection) as a Markdown spec with ⌘⇧C, or `shape prompt FILE --target build`, and hand it to Claude, ChatGPT or your coding agent to build.

A board is a plain text file in your repository, so it diffs, merges and reviews like code. There is no account, no server and no new bill. The command is `shape` (`diagram` still works, from before the rename) and boards are `.shape` files (`.diagram` ones still open); the engine is Rust compiled to WebAssembly, so the CLI, the editor, the embeddable library and the Rust crate share it.

**[Try it in your browser](https://dobrinov.github.io/shape/app/)** (no install needed) · [Website](https://dobrinov.github.io/shape/) · [Examples](https://dobrinov.github.io/shape/examples/) · [Language reference](crates/core/LANGUAGE.md)

![The Shape editor: a mindmap, a kanban board, a sequence diagram and a decision record on one board](assets/screenshot.png)

```shape
title: Checkout
user: Customer [person]
group backend: Backend [blue] {
  api: Orders API [detail: Rust · Axum]
  db: Postgres [cylinder]
}
user -> api: POST /orders
api -> db
api ..> mail: receipt

comment on api by deyan: Should payments be a separate service?
reply c1 by ana: Not before launch.
```

```bash
shape docs/checkout.shape                        # open the editor; it saves to the file as you work
shape prompt docs/checkout.shape --target code # open comments → a task for your coding agent
shape apply docs/checkout.shape reply.md         # apply the LLM's reply (ops JSON or a whole diagram)
shape png docs/checkout.shape --base main        # PNG of what this branch changed
```

## Mindmaps

Press **T** and click, type the central topic, then keep typing: **Tab** adds a branch, **Enter** a sibling, **⇧Tab** outdents (on the centre it adds a new centre above it), arrows walk the map, **⌥↑/↓** reorders, **.** folds. Drag a topic onto another to move its branch. Paste a Markdown list to get a map. Maps sit on the same board as boxes, stickies and arrows, and topics can be linked and commented on like anything else.

```shape
mindmap offsite: Team offsite [layout: both] {
  Venue
    Who books it?
  Budget [red]
    Travel and hotels
  Prompt: build checkout
    Context
    Constraints
}
```

**✦ AI that costs few tokens.** *Grow this topic* sends a model only the map as an outline plus the path to the topic, and gets indented lines back, about 5–15 tokens per topic. It runs on the AI subscription you already have, through its CLI: `shape FILE --ai claude` (Claude Code; `claude:sonnet` picks a model), `--ai codex` (Codex CLI) or `--ai gemini` (Gemini CLI); by default the first one installed. The editor takes any `ai(prompt) → Promise<string>` function too, and without one it shows the prompt to copy and takes the reply back. With none connected, *✦ → Connect your AI* says how to install and sign in to one; when one fails (not signed in, a limit reached, not found), the editor shows what it said and what to try, and *Your AI: check it* tests it. The CLIs are also looked for where their installers put them, so a board opened from Finder finds them too.

**Big boards.** Drag a section by its frame and it keeps its place (`group a: A [x: 400, y: 120]`), so adding a section doesn't reshuffle the board. ⌘F finds boxes, topics, rows and sections by their text (Enter for the next). Zoomed out, section titles stay readable. *Present* (the screen icon in the toolbar) shows the board's sections one at a time in reading order (← →, Esc).

**✦ The AI button.** The round button in the bottom-right corner holds the AI. *Ask* answers a question about the board (all of it, canvases included) by drawing a small diagram for it on a temporary canvas: a summary on top, then boxes, sections, a timeline or a table, each piece linked back to its place on the board. Nothing is saved: follow up, comment on it, **Save as canvas** to keep it, or **Rethink the board from this** to bring the board in line with the answer. *Add what you know* (⌘K) takes details or information you write down (or a change you ask for) about the selection or the whole canvas, and sends it with just that part of the board; the model answers with edits, a reply that does not apply is sent back with its error (twice at most), and you see the changes highlighted before you keep or discard them. On the command line: `shape ask FILE "put the storage in a section and add a cache" [--on ID,ID] [--dry-run]`.

**Recipes.** Ready-made arrangements of blocks for what a software team keeps coming back to: Achievements, Mistakes and lessons, Incidents (log, timeline, post-mortems), a traffic-light Status report, Burndown, OKRs, Roadmap, Retrospective, RACI, Prioritisation, Personas, User journey, Tech debt, Release checklist, Stakeholder map, Team ownership and Metrics. They are not in the Blocks palette: the AI knows them, so telling *Add what you know* "payments were down for 2 hours on Monday" adds an incident in the Incidents shape. Right-click the empty board → *Insert recipe…* places one. Add your team's own as `.shape` files in a `recipes/` folder of the repository, the first line saying `# recipe: Name — when to use it`.

**Perspectives.** One board, seen by different audiences. A board's type (`type: product` or `type: project`, chosen by its template or in the Board popover) brings built-in perspectives: Product, Technical, User and Senior management for a product; Senior management, Team, Product and Technical for a project; and Legal, Finance, Marketing, Commercial and Design for both. Pick one in the top bar and the board shows only what that audience needs: the elements tagged for it, the blocks it cares about, titles only for management. Add your own (*New perspective…*), and let the AI tag the board (*✦ Sort into perspectives*). It only changes what you see, never the file. `shape new FILE --template product|project` starts one.

**✦ Rethink.** After a round of collecting (comments from teammates, interviews, commits on a branch), *Rethink* (the AI button, or right-click on the empty board) brings the whole board up to date, every canvas included. Open comments come first: the AI changes the board everywhere a comment affects it, then resolves the comment with what it changed. Then it organises: it groups related items, merges duplicates and updates what newer information changed. It keeps every piece of information and every id. When the board is in git, what changed since the branch left `main` (or since the last commit) counts as new information too. As with *Add what you know*, you see the changes and the AI's note before keeping them; when it changes nothing, its note says why.

**To-do lists.** *Blocks → To-do list* makes a checklist (`[x] Release notes [owner: ana, due: 28 Oct]`, sub-items indented) with a progress bar; click a box to tick it, overdue items show red, and the AI ticks and adds items too.

**Wireframes.** *Blocks → Phone screen* or *Web page* draws a screen as a phone, a tablet or a browser window, written as text, one component per line (`button submit: Sign in [blue]`, `tabs: Home | Search | Me`). There are about 40 kinds: headers, navbars, fields, buttons, lists, tables, tabs, tab bars, sidebars, cards, images, maps, charts, steppers, alerts and more. Drag from a button to another screen to draw the flow; dropped on empty canvas, it makes the next screen.

**Presentations.** The Present button opens presentation mode: the board's slides on the right (each a section, block, mindmap, canvas or the whole board), their order drawn on the board, with a title, speaker notes and a transition per slide, and instructions for the presentation that the AI follows when you ask it to *draft the slides*. Start from the board's sections in one click. *▶ Present* plays it full screen. It is all in the file, as a `presentation` block.

**AI prompts.** *Board → AI prompts…* shows what Rethink, Sort into perspectives, Ask and Add what you know send for the board as it is now, to read or copy. Nothing is sent from there.

**History.** The clock button in the top bar lists the board's versions: the commits that changed it, or, for a board git does not track, versions kept beside it in `.shape-history/` (one at most every five minutes while you save, signed by whoever saved). Hover a version for its exact time. *Changes* highlights what changed since it; *Restore* brings it back (⌘Z undoes that). When git does not know your name, the editor asks for it before you edit or comment.

**Mindmaps as prompts.** *Copy as prompt* (or `shape outline FILE --md`) turns a map into Markdown: the centre is the title and topics are nested bullets; when every main branch has something under it, main branches become `##` sections. It works on any selection too (right-click or `⌘⇧C`): sections become headings with their blocks as bullets, followed by the arrows between them and their open comments. Swimlanes read as numbered steps in flow order (decisions say which step each answer leads to), gantt tasks with their start and end dates. A `#id` link pulls the block it points at into the prompt, so a prompt map can reference a flow or a plan on the same board.

## Why

Whiteboard tools are great for talking. Their output, though, ends up in a format that neither git nor an LLM can work with. Shape keeps the friendly parts (drag boxes, drop sticky notes, comment on anything) and stores the result as **text you can diff, review and feed to an agent**.

- **Two languages, one model.**
  - The **DSL** is for people: one statement per line, so it reads well in a PR.
  - The **JSON model** (with a JSON Schema) and **ops** are for programs and LLMs: strict, validated edits that are applied all at once or not at all.
  - **Mermaid** can be imported and exported. Everything converts to everything else without loss (`shape convert`).
- **A real editor.**
  - Double-click to add a box.
  - Drag a node's dot onto another node to connect them, or onto empty space to create a new connected node.
  - Sticky notes, titled sections (`⌘G`; drag blocks in or out), multi-select, duplicate, arrow-key nudging, an inspector, undo and redo.
  - A live code pane (DSL, JSON or Mermaid) that stays in sync with the canvas both ways.
  - Light and dark themes, plus pan and zoom.
- **Flowcharts, swimlanes and gantt charts.**
  - Boxes, decisions and 11 more shapes, with labelled, dashed or reversed arrows (right-click a block or an arrow).
  - Swimlanes (`lanes { lane … }`): bands the flow runs across, lined up step by step.
  - Gantt charts (`gantt { Design [5d]  Build [2w, after: design] }`): a timeline by day, week, month or quarter. Drag a bar to reschedule it, drag its end to change its duration, draw an arrow between bars to make one wait for the other. Milestones, progress and owners; a forecast from today shows late work and everything it pushes back (`shape schedule FILE` lists it).
  - Database tables (`entity users { id: uuid [pk]  team_id: uuid [fk: teams] }`): columns with types and keys, foreign keys drawn between columns, and schema changes reviewed column by column (`shape diff`, `--base`).
  - Database schemas (`schema db: Database [file: db/structure.sql] { … }`): the tables of a Postgres `structure.sql`, a MySQL or SQLite dump or a Rails `schema.rb`, read by [schema](https://github.com/dobrinov/schema)'s parser and kept up to date from the file (`shape schema BOARD db/structure.sql`, or *Blocks → Database schema* in the editor).
  - Tables (`table { option | cost  Local | $0 }`): a header and rows of cells that wrap; rows can be commented on, linked and tinted. *Edit as text…* in the editor.
  - Themes (`theme: paper`, or your own `theme brand { … light { … } dark { … } }`, also from a shared `./brand.theme` file): colours, named colours, font and corner radius, each in a light and a dark mode that follows the viewer, for the board, its exports and the editor canvas. Pick one or *Edit theme…* in the toolbar.
  - Charts (`chart [bar] { quarter | EU | US  Q1 | 120 | 80 }`): bars (stacked, horizontal), lines, areas, pie, donut and scatter from a small table that diffs like text. Hover a bar for its values; *Edit data…* in the editor.
  - Risk matrices (`risks { Contract slips [p: 4, i: 4, detail: "…"] }`): probability × impact on a heat-map grid, each risk a numbered dot with its description on hover. Drag a dot to another cell to re-rate it.
- **Links between elements.** Right-click a block → *Link to…* and click another element; the block's link badge takes you there (`link: #id`). Web links work the same way.
- **Comments become prompts.**
  - Press **C** and click a block (or empty canvas) to start a thread, right where you clicked. Threads support replies, resolving and reopening.
  - **Prompt** builds a task from the open threads. You choose what the agent should do: *update the diagram*, *change the code* or *discuss*. You also choose the reply format: ops or a whole diagram.
  - **Apply reply** takes the LLM's answer as it is (fenced JSON or a ```` ```shape ```` block), applies it atomically and resolves the comments with a note on what was done.
- **Export anywhere.**
  - Images: SVG and PNG. PNG is rasterised by resvg in the CLI and by a canvas in the browser.
  - Documents: a self-contained interactive HTML page.
  - For pasting: Mermaid (for GitHub Markdown) and a Markdown *brief* that gives an LLM the diagram as context (the source with every id, plus gantt dates and swimlane steps in order).
  - A **share link** that carries the whole diagram, compressed, in the URL.
- **Git-aware.** `--base main` (or any ref or file) highlights what changed: green for added, red for removed, amber for changed. The editor's *Compare ▾* button does the same against any git version.
- **Usable as a library.** Use the Rust crate `shape-core` on servers. In the browser, use one `shape.embed.js` file (JS with the WASM inlined) to render diagrams or mount the full editor. Agent skills teach Claude Code to draw, edit and review diagrams.

## Install

**Homebrew** (macOS, Linux):

```bash
brew install dobrinov/tap/shape
```

Had it installed as `diagram` (its name before October 2026)? Run `brew migrate diagram` once, then `brew upgrade shape` as usual.

**Binaries:** download an archive for macOS (Apple silicon or Intel), Linux (x86_64 or arm64) or Windows (x86_64) from the [latest release](https://github.com/dobrinov/shape/releases/latest), unpack it and put `shape` on your `PATH` (the archive also holds `diagram`, the old name, as an alias). `SHA256SUMS` lists the checksums.

**From source:** you need Rust with the `wasm32-unknown-unknown` target, and `wasm-pack`.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
git clone https://github.com/dobrinov/shape && cd shape
make install        # builds web/pkg (WASM), then `cargo install --path crates/cli`
```

A plain `cargo install --path crates/cli` also works. Its `build.rs` runs wasm-pack whenever the WASM bundle is missing or out of date. The editor's native window uses the system webview (WebKit on macOS, WebView2 on Windows 10/11, WebKitGTK on Linux: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev` to build there); `--no-default-features` builds without it, and boards then open in the browser. The Linux release binaries are built that way, so they run without WebKitGTK; build from source for the window on Linux.

**The editor:** `shape FILE` opens the board in its own window; closing the window saves and stops. `shape` with no file asks where to save a new board. `--browser` opens a browser tab instead (as before), `--no-open` just runs the server. Where no window can be made (no display), the browser opens.

**Open boards from Finder (or Files, or Explorer):**

```bash
shape install-app        # once; `shape install-app --remove` undoes it
```

After this, `.shape` files get their own document icon, and double-clicking one opens it in the editor's window (opening Shape.app itself starts a new board). Closing the window stops it; where it falls back to a browser tab, the server stops by itself a few minutes after you close the tab. On macOS this is a small Shape.app in `~/Applications` (or `~/Library/Application Support/shape`). On Linux it is a MIME type, an icon and a `.desktop` entry in `~/.local/share`. On Windows it is a file association for your user. Run it again after moving or reinstalling Shape.

**New versions:** once a day, the editor asks GitHub whether a newer release exists. If one does, the terminal says so (with `brew upgrade shape` when Homebrew installed it), and a *New* badge in the top bar links to the release notes. Set `SHAPE_NO_UPDATE_CHECK=1` to turn this off; it is also off when `CI` is set.

## Usage

```
shape [FILE]                       open FILE in the editor, in its own window (created if
                                   missing; no FILE asks where to save a new board)
                                   --browser: in a browser tab · --no-open: the server only
shape new FILE [--template flow|architecture|board]

shape svg  FILE [-o out.svg]       static SVG       ┐ --dark --transparent --title
shape png  FILE [-o out.png]       PNG (--scale 2)  │ --comments open|all|none
shape html FILE [-o out.html]      interactive page ┘ --base FILE|GIT_REF  (highlight changes)
shape convert FILE --to dsl|json|mermaid|brief
shape brief FILE                   Markdown description for LLM context

shape comments FILE [--all] [--json]
shape comment  FILE [--on ID] [--by NAME] TEXT
shape reply      FILE COMMENT_ID TEXT
shape resolve  FILE COMMENT_ID [--note TEXT]
shape prompt     FILE [--target build|diagram|code|discuss] [--reply ops|dsl] [--comment c1,c2] [--all] [--context auto|near|section|all]
shape apply      FILE [PATCH|-] [--dry-run]

shape outline FILE [ID] [--md]     shape expand FILE ID [--ask TEXT] | --new TOPIC   (--ai CMD)
shape check FILE...     shape fmt FILE... [--check]   shape diff OLD NEW|GIT_REF
shape spec [--json-schema]     shape embed [-o shape.embed.js]   shape skills install [--global]
```

`shape FILE` starts a local server (from port 5591) and opens the editor. Every change is saved to the file. If something else edits the file, such as your `$EDITOR` or an agent, the editor reloads it within a second or two. Press `⌘S` to save right away.

### Editor shortcuts

The minimap in the bottom-left corner shows the whole board; click or drag in it to move around, **M** hides it.

| | |
|---|---|
| `V` `T` `N` `S` `C` `M` | select · mindmap · box · sticky · connect · comment |
| `Tab` · `Enter` · `⇧Tab` · `F2` · `.` | mindmap: branch · sibling · outdent · rename · fold |
| double-click | add a box / edit a label |
| drag a node's dot | connect, or drop on empty space to create a connected node |
| `⌘G` · `⌘D` · `⌘A` | section · duplicate · select all |
| `Enter` · `Delete` · arrows | edit label · delete · nudge (`⇧` for ×5) |
| `⌘Z` · `⇧⌘Z` | undo · redo |
| `F` · `0` · `+` `-` · space+drag | fit · 100% · zoom · pan |
| `⌘/` · `?` | toggle code · show shortcuts |

## The language

The full reference is in [LANGUAGE.md](crates/core/LANGUAGE.md), and `shape spec` prints it. The short version:

| | |
|---|---|
| `id: Label [attrs]` | node. Attrs: `shape`, `color`, `detail`, `link`, `tags`, `x`/`y` (pinned) |
| `a -> b: label` | edge. `->` solid, `..>` dashed, `=>` thick, `--` / `..` / `==` no arrow, `<->` both |
| `a -> b -> c`, `a -> b, c` | chains and fan-out |
| `group id: Label [color] { … }` | groups, which can be nested |
| `mindmap` · `lanes` · `kanban` · `sequence` · `gantt` · `risks` · `table` · `chart` · `entity` · `schema` · `adr` · `timeline` · `cycle` `id: Title { … }` | blocks: mindmaps, swimlanes, kanban boards, sequence diagrams, gantt charts, risk matrices, tables, charts, database tables, database schemas (from a file), decision records, timelines, cycles |
| `link: other.shape#id` | a link to an element on another board |
| `comment [id] on TARGET by WHO: text` · `reply id: text` | review threads; add `[resolved, resolution: …]` to close one |
| `title:` `direction:` `layout:` `routing:` `theme:` | settings |

The 13 shapes are box, rounded, pill, circle, diamond, hexagon, cylinder, parallelogram, document, person, cloud, sticky and text. There are 9 named colours, and any `#hex` value also works.

For LLMs there is an **ops** language for precise edits:

```json
{"ops": [
  {"op": "add_node", "id": "cache", "label": "Redis", "shape": "cylinder", "group": "backend"},
  {"op": "add_edge", "from": "api", "to": "cache", "label": "reads"},
  {"op": "rename", "id": "api", "to": "orders-api"},
  {"op": "resolve_comment", "id": "c1", "resolution": "Added a read-through cache"}
]}
```

## The review loop

1. **Draw:** `shape docs/checkout.shape`, or ask Claude Code to draw it (see *Agent skills*).
2. **Review:** colleagues open the same file (or a share link, or an HTML export) and leave comments.
3. **Prompt:** use the **Prompt** button or `shape prompt FILE --target code`. The prompt contains the diagram, each open thread with its replies, and exact instructions for replying. On a big board it shows only the sections the comments are about (`--context near|section|all` to choose), so one comment doesn't cost the whole board in tokens.
4. **Apply:** paste the reply into **Apply reply** or run `shape apply FILE reply.md`. The changes are applied atomically, and each comment is resolved with a note on what was done. Only the statements that changed are rewritten: the rest of the file, its blank lines and `#` notes stay as you wrote them, so the commit diff shows just the change. Comments on deleted elements are kept as canvas comments, so feedback is never lost silently.

## Using it as a library

### Browser (any app, no build step)

```bash
shape embed -o public/shape.embed.js
```

```html
<script src="/shape.embed.js"></script>
<div id="board" style="height:600px"></div>
<script>
  const editor = await Shape.mount("#board", {
    source,                      // DSL, JSON or Mermaid
    mode: "edit",                // or "view"
    author: currentUser.name,
    onChange: (src) => save(src),
  });
  editor.apply({ ops: [...] });                 // programmatic edits
  editor.setBase(previousVersion);              // highlight changes
  const prompt = editor.prompt({ target: "code" });
  const svg = await Shape.render("a -> b", { theme: "dark" });
  Shape.renderAll("pre > code.language-shape");   // render ```shape fences in Markdown
</script>
```

`<div data-shape data-mode="view">…source…</div>` mounts itself. If you serve `web/` and `web/pkg` yourself, load `pkg/shape_wasm.js`, `editor.js` and `editor.css`, then call `DiagramEditor.create(el, { wasm: wasm_bindgen, … })`.

### JavaScript / TypeScript (Node and browsers)

`js/` is a typed ES-module package around the WASM build, for tools built on top of Shape: parse, render, apply ops or a model's reply, prompts, mindmap outlines, diffs. Build it with `make js`; see [js/README.md](js/README.md).

```js
import { init, Board } from "./js/index.mjs";
await init();
const board = Board.parse("api -> db");
board.apply(modelReply);                       // ```json {"ops": [...]}``` → atomic edit, throws DiagramError
const { svg } = board.render({ theme: "dark" });
```

### Rust

```toml
[dependencies]
shape-core = { git = "https://github.com/dobrinov/shape" }
```

```rust
let mut doc = shape_core::parse(&source)?;                  // DSL, JSON or Mermaid
let svg = shape_core::render_svg(&doc, &Default::default());
let ops = shape_core::ops::parse_ops(&llm_reply)?;          // tolerant of chat formatting
shape_core::ops::apply(&mut doc, &ops)?;                    // atomic
let prompt = shape_core::prompt::prompt(&doc, &Default::default());
let text = shape_core::dsl::print(&doc);                    // store this
```

### In SpecDeck

SpecDeck is already a Rust server with a no-build JS front end and a WASM canvas, so Shape fits in without adding any new tooling:

- **Server:** add `shape-core` as a git dependency of `crates/server`. Store diagrams as DSL text in a `diagrams` table, or as `.shape` files in the project repo. To include review feedback in the prompt composer, call `shape_core::prompt::prompt(&doc, &PromptOptions { target: Target::Code, file: Some(path), .. })` alongside Spec Kit's own review comments.
- **Web:** copy `shape.embed.js` into `web/` (or `make embed`) and mount an editor on a board section: `Shape.mount(el, { source, author, onChange: (src) => api.put(…) })`. Editor `change` events map naturally onto SpecDeck's SSE refresh.
- **Agents:** run `shape skills install` in the project repo, so the harnesses SpecDeck launches can read, edit and resolve shape comments with the CLI.

## Agent skills

```bash
shape skills install              # into .claude/skills of the current repo
shape skills install --global     # into ~/.claude/skills
```

Installed skills follow upgrades: the first time a newer Shape runs, it updates the ones it installed, unless you edited them (`SHAPE_NO_SKILL_REFRESH=1` turns this off).

| Skill | What it does |
|---|---|
| `shape-draw` | Draw an architecture, flow or board diagram (from a description or by reading the codebase), validate it and render it |
| `shape-review` | Act on open comments: change the code or the diagram, then resolve or reply to each thread |
| `shape-edit` | Make precise structural edits with ops (renames, regrouping, removals) |
| `shape-embed` | Use the JS bundle or the Rust crate in another project |
| `shape-mindmap` | Create and grow mindmaps with outlines and `add_outline`, and turn them into prompts |

## Repository layout

| Path | What it is |
|---|---|
| `crates/core` | Model, DSL parser and printer, JSON (+ schema), Mermaid, ops, layout (layered Sugiyama with compound groups, and grid), edge routing, SVG renderer, diff, prompts |
| `crates/wasm` | `wasm-bindgen` bindings (`Diagram` class and one-shot helpers) |
| `crates/cli` | The `shape` binary: local editor server, exports (PNG via resvg), review commands, embed bundle, skills |
| `web/` | No-build editor component (`editor.js`, `editor.css`), app shell (`app.js`) and embed API (`embed.js`). `web/pkg` is generated. |
| `js/` | Typed JS/TS package for Node and browsers (`make js`; `js/pkg` is generated) |
| `skills/` | Agent skills shipped inside the binary |
| `scripts/e2e/` | Browser tests of the editor (Playwright, `make e2e`) |
| `site/`, `scripts/build-site.sh` | GitHub Pages: landing page, playground and examples (`make site` → `docs/`) |

```bash
make test     # cargo tests + node tests of the WASM API (and of js/ once `make js` has run)
make js       # build the js/ package's WASM into js/pkg
make e2e      # browser test of the editor (uses the installed Chrome)
make dev      # serve web/ from disk while hacking on the editor
make site     # build the static site into docs/
```

## Limitations

- Elbow arrows go round the boxes in their way; curved and straight ones (`routing: curved | straight`) still go straight across.
- `#` comment lines in a `.shape` file are dropped when a tool rewrites the file (canvas edits, `apply`, `fmt`). Use `comment` statements for anything that must survive.
- Collaboration happens through files, git, share links and HTML exports. There is no real-time multi-user server (yet).

## License

MIT
