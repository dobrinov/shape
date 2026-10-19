//! Perspectives: ways of looking at one board. A perspective picks which
//! elements to show (those tagged for it, and the blocks of the kinds it
//! names) and whether to show their details; the board itself is not
//! changed. The board's type (`type: product`) decides the built-in ones;
//! `perspective id: Name [tags: …, show: …, details: off]` adds one, or
//! overrides a built-in with the same id.
use crate::model::{BoardKind, Document, Group, Node, Perspective};

/// The block kinds `show` can name.
pub const KINDS: &[&str] = &[
    "mindmap", "timeline", "adr", "risks", "kanban", "gantt", "table", "chart", "entity",
    "sequence", "lanes", "sticky", "cycle", "screen", "todo",
];

fn p(id: &str, label: &str, tags: &[&str], show: &[&str], details: bool) -> Perspective {
    Perspective {
        id: id.into(),
        label: label.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        show: show.iter().map(|t| t.to_string()).collect(),
        details,
    }
}

/// The perspectives a board of this type comes with. Each starts high
/// level (see [`fold`]): its sections and blocks folded into summary cards,
/// except its headline blocks ([`headline`]).
pub fn builtins(kind: BoardKind) -> Vec<Perspective> {
    let product = p(
        "product",
        "Product",
        &["product"],
        &["mindmap", "timeline", "screen", "chart", "kanban", "adr"],
        true,
    );
    let tech = p(
        "tech",
        "Technical",
        &["tech"],
        &["entity", "sequence", "lanes", "adr", "cycle"],
        true,
    );
    let user = p(
        "user",
        "Users",
        &["user"],
        &["screen", "lanes", "sticky", "mindmap"],
        true,
    );
    let delivery = p(
        "delivery",
        "Delivery",
        &["delivery", "team"],
        &["timeline", "todo", "gantt", "kanban", "risks", "table"],
        true,
    );
    // the business around it, on product and project boards alike
    let around = || {
        vec![
            p(
                "legal",
                "Legal",
                &["legal"],
                &["adr", "risks", "table"],
                true,
            ),
            p(
                "business",
                "Business",
                &["business", "finance", "commercial"],
                &["chart", "table", "timeline"],
                true,
            ),
            p(
                "marketing",
                "Marketing",
                &["marketing"],
                &["timeline", "chart", "mindmap", "screen"],
                true,
            ),
            p(
                "design",
                "Design",
                &["design"],
                &["screen", "lanes", "mindmap", "sticky"],
                true,
            ),
        ]
    };
    match kind {
        BoardKind::Sketch => Vec::new(),
        BoardKind::Product => {
            let mut v = vec![product, tech, user];
            v.extend(around());
            v
        }
        BoardKind::Project => {
            let mut v = vec![delivery, product, tech];
            v.extend(around());
            v
        }
    }
}

/// What a built-in perspective is for, in a line (for the picker).
pub fn purpose(id: &str) -> Option<&'static str> {
    Some(match id {
        "product" => "What we build and why: the goals, the plan, the screens",
        "tech" => "How it is built: the data, the services, the flows",
        "user" => "What people see and do: screens, journeys, feedback",
        "delivery" => "Who does what by when, and what could stop it",
        "legal" => "Obligations, decisions and risks",
        "business" => "Money, pricing, deals and the numbers",
        "marketing" => "The launch, the message and the dates",
        "design" => "Screens, flows and the user journey",
        _ => return None,
    })
}

/// The blocks a perspective opens with unfolded: the high-level picture
/// that role looks at first (a chart for the business, the timeline for
/// delivery). The rest starts folded.
pub fn headline(id: &str) -> &'static [&'static str] {
    match id {
        "business" => &["chart"],
        "delivery" | "marketing" | "product" => &["timeline"],
        _ => &[],
    }
}

/// Every perspective of a board: the built-in ones of its type, with the
/// ones written on it overriding (same id) or added after them.
pub fn all(doc: &Document) -> Vec<Perspective> {
    let mut out = builtins(doc.settings.kind);
    for own in &doc.perspectives {
        match out.iter_mut().find(|x| x.id == own.id) {
            Some(x) => *x = own.clone(),
            None => out.push(own.clone()),
        }
    }
    out
}

fn kind_of(g: &Group) -> &'static str {
    if g.adr.is_some() {
        "adr"
    } else if g.risks {
        "risks"
    } else if g.timeline {
        "timeline"
    } else if g.gantt {
        "gantt"
    } else if g.kanban {
        "kanban"
    } else if g.table {
        "table"
    } else if g.chart.is_some() {
        "chart"
    } else if g.entity {
        "entity"
    } else if g.sequence {
        "sequence"
    } else if g.lanes {
        "lanes"
    } else if g.cycle {
        "cycle"
    } else if g.screen.is_some() {
        "screen"
    } else if g.todo {
        "todo"
    } else {
        "section"
    }
}

/// The board as `p` sees it: only what it shows, details off when it
/// says so. Sections stay around what they still hold; arrows and comments
/// stay with what they join or are about.
pub fn view(doc: &Document, p: &Perspective) -> Document {
    let tagged = |tags: &[String]| tags.iter().any(|t| p.tags.contains(t));
    // a block shows whole when it is of a kind the perspective shows
    let whole = |g: &Group| p.show.iter().any(|k| k == kind_of(g));
    let group_shown_whole = |id: &str| -> bool {
        let mut cur = doc.group(id);
        let mut hops = 0;
        while let Some(g) = cur {
            if whole(g) {
                return true;
            }
            hops += 1;
            if hops > 64 {
                break;
            }
            cur = g.parent.as_deref().and_then(|p| doc.group(p));
        }
        false
    };
    // a mindmap topic goes with its centre
    fn centre_of<'a>(doc: &'a Document, n: &'a Node) -> Option<&'a Node> {
        let mut cur = n;
        let mut hops = 0;
        while let Some(pid) = cur.parent.as_deref() {
            cur = doc.node(pid)?;
            hops += 1;
            if hops > 256 {
                break;
            }
        }
        (cur.mindmap.is_some()).then_some(cur)
    }
    let node_shown = |n: &Node| -> bool {
        if tagged(&n.tags) {
            return true;
        }
        if n.group.as_deref().is_some_and(group_shown_whole) {
            return true;
        }
        if let Some(c) = centre_of(doc, n) {
            if tagged(&c.tags) || p.show.iter().any(|k| k == "mindmap") {
                return true;
            }
            if c.group.as_deref().is_some_and(group_shown_whole) {
                return true;
            }
        }
        n.shape == crate::model::Shape::Sticky && p.show.iter().any(|k| k == "sticky")
    };
    let mut v = doc.clone();
    v.nodes.retain(|n| node_shown(n));
    // groups: shown whole, or holding something shown (their parents too)
    let mut keep: Vec<String> = doc
        .groups
        .iter()
        .filter(|g| group_shown_whole(&g.id))
        .map(|g| g.id.clone())
        .collect();
    for n in &v.nodes {
        let mut cur = n.group.as_deref();
        while let Some(gid) = cur {
            if keep.iter().any(|k| k == gid) {
                break;
            }
            keep.push(gid.to_string());
            cur = doc.group(gid).and_then(|g| g.parent.as_deref());
        }
    }
    v.groups.retain(|g| keep.contains(&g.id));
    let present = |id: &str| v.nodes.iter().any(|n| n.id == id) || keep.iter().any(|k| k == id);
    let edges: Vec<_> = doc
        .edges
        .iter()
        .filter(|e| present(&e.from) && present(&e.to))
        .cloned()
        .collect();
    let comments: Vec<_> = doc
        .comments
        .iter()
        .filter(|c| {
            c.on.as_deref()
                .is_none_or(|o| present(o) || edges.iter().any(|e| e.id == o))
        })
        .cloned()
        .collect();
    v.edges = edges;
    v.comments = comments;
    if !p.details {
        for n in &mut v.nodes {
            n.detail = None;
        }
    }
    v
}

/// `t` cut to about `max` characters at a word, with an ellipsis.
fn short(t: &str, max: usize) -> String {
    if t.chars().count() <= max {
        return t.to_string();
    }
    let cut: String = t.chars().take(max).collect();
    let cut = cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut);
    format!("{}…", cut.trim_end_matches([',', ':', '·', ' ']))
}

/// The id of the card standing for folded block `gid`.
pub fn fold_id(gid: &str) -> String {
    format!("@fold/{gid}")
}

/// What a folded block holds, in a line: what is in it and its first few
/// items (`6 risks, 2 high · Fraud reporting, Apple review…`).
pub fn summary(doc: &Document, g: &Group) -> String {
    let members: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
        .collect();
    let inside: Vec<&Group> = doc
        .groups
        .iter()
        .filter(|x| x.parent.as_deref() == Some(g.id.as_str()))
        .collect();
    let n = members.len();
    let plural =
        |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    let what = if g.risks {
        let levels = crate::risks::levels(g);
        let high = members
            .iter()
            .filter(|m| {
                crate::risks::rating(m, levels)
                    .is_some_and(|(p, i)| u16::from(p) * u16::from(i) >= u16::from(levels) * 3)
            })
            .count();
        let mut s = plural(n, "risk", "risks");
        if high > 0 {
            s.push_str(&format!(", {high} high"));
        }
        s
    } else if g.kanban {
        inside
            .iter()
            .map(|c| {
                let k = doc
                    .nodes
                    .iter()
                    .filter(|m| m.group.as_deref() == Some(c.id.as_str()))
                    .count();
                format!("{k} {}", c.display().to_lowercase())
            })
            .collect::<Vec<_>>()
            .join(", ")
    } else if g.todo {
        let done = members.iter().filter(|m| crate::todo::is_done(m)).count();
        format!("{done} of {n} done")
    } else if g.gantt {
        plural(n, "task", "tasks")
    } else if g.timeline {
        plural(n, "event", "events")
    } else if g.table {
        plural(n, "row", "rows")
    } else if g.chart.is_some() {
        "a chart".into()
    } else if g.entity {
        plural(n, "column", "columns")
    } else if let Some(st) = &g.adr {
        format!("decision, {st}")
    } else if let Some(d) = g.screen {
        format!(
            "{} screen, {}",
            d.as_str(),
            plural(n, "component", "components")
        )
    } else if g.sequence {
        plural(n, "participant", "participants")
    } else if g.cycle {
        plural(n, "step", "steps")
    } else if g.lanes {
        plural(inside.len(), "lane", "lanes")
    } else {
        let mut parts = vec![plural(n, "item", "items")];
        if !inside.is_empty() {
            parts.push(plural(inside.len(), "section", "sections"));
        }
        parts.join(", ")
    };
    // a decision says what was decided; anything else names a few items
    let gist: Vec<String> = if g.adr.is_some() {
        members
            .iter()
            .find(|m| m.display() == "decision")
            .and_then(|m| m.detail.clone())
            .into_iter()
            .collect()
    } else if g.chart.is_some() || g.screen.is_some() {
        Vec::new()
    } else {
        members
            .iter()
            .filter(|m| m.parent.is_none())
            .map(|m| m.display().replace('\n', " "))
            .chain(inside.iter().map(|c| c.display().to_string()))
            .take(3)
            .map(|t| short(&t, 28))
            .collect()
    };
    let mut line = what;
    if !gist.is_empty() {
        let more = if n + inside.len() > gist.len() {
            "…"
        } else {
            ""
        };
        line.push_str(&format!(" · {}{more}", gist.join(", ")));
    }
    line
}

/// A perspective's high-level view: every top-level section and block not
/// in `open` (or a headline kind of `p`) folded into a card with its title
/// and [`summary`] (id [`fold_id`]); arrows to anything inside go to the
/// card, mindmaps fold to their centre.
pub fn fold(doc: &Document, p: &Perspective, open: &[String]) -> Document {
    let keep = |g: &Group| open.contains(&g.id) || headline(&p.id).contains(&kind_of(g));
    let mut out = doc.clone();
    let mut moved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for g in doc.groups.iter().filter(|g| g.parent.is_none()) {
        if keep(g) {
            continue;
        }
        let card = fold_id(&g.id);
        // what is inside, at any depth
        let mut groups = vec![g.id.clone()];
        let mut k = 0;
        while k < groups.len() {
            let cur = groups[k].clone();
            groups.extend(
                doc.groups
                    .iter()
                    .filter(|x| x.parent.as_deref() == Some(cur.as_str()))
                    .map(|x| x.id.clone()),
            );
            k += 1;
        }
        for id in &groups {
            moved.insert(id.clone(), card.clone());
        }
        for n in doc
            .nodes
            .iter()
            .filter(|n| n.group.as_ref().is_some_and(|x| groups.contains(x)))
        {
            moved.insert(n.id.clone(), card.clone());
        }
        let mut c = Node::new(&card);
        c.label = g.display().to_string();
        c.detail = Some(summary(doc, g));
        c.color = g.color.clone();
        c.shape = crate::model::Shape::Rounded;
        (c.x, c.y) = (g.x, g.y);
        out.nodes.push(c);
    }
    // a topic's own topics go with it: a mindmap folds to its centre
    for n in out.nodes.iter_mut() {
        if n.mindmap.is_some() && n.group.is_none() && !open.contains(&n.id) {
            n.collapsed = true;
        }
    }
    out.nodes
        .retain(|n| !moved.contains_key(&n.id) || n.id.starts_with("@fold/"));
    out.groups.retain(|g| !moved.contains_key(&g.id));
    let to = |id: &str| moved.get(id).cloned().unwrap_or_else(|| id.to_string());
    let mut seen = std::collections::HashSet::new();
    out.edges = doc
        .edges
        .iter()
        .filter_map(|e| {
            let (a, b) = (to(&e.from), to(&e.to));
            if a == b || !seen.insert((a.clone(), b.clone())) {
                return None;
            }
            let mut e2 = e.clone();
            // an arrow that now joins two cards says what it joined
            if a != e.from || b != e.to {
                e2.id = format!("{a}->{b}");
            }
            (e2.from, e2.to) = (a, b);
            Some(e2)
        })
        .collect();
    for c in out.comments.iter_mut() {
        if let Some(on) = c.on.clone() {
            c.on = Some(to(&on));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;

    #[test]
    fn types_and_perspectives_round_trip_and_change_by_ops() {
        let src = "title: Spend Cards\ntype: project\n\nperspective exec: Leadership [tags: exec boss, show: adr risks, details: off]\nperspective ux: Design [tags: ux]\n\napi: Orders API [tags: tech]\n";
        let doc = dsl::parse(src).unwrap();
        assert_eq!(doc.settings.kind, BoardKind::Project);
        assert_eq!(dsl::print(&doc), src);
        assert_eq!(crate::from_json(&crate::to_json(&doc)).unwrap(), doc);
        // a box called type stays a box
        let d = dsl::parse("type: Database\n").unwrap();
        assert_eq!(
            (d.settings.kind, d.node("type").map(|n| n.label.as_str())),
            (BoardKind::Sketch, Some("Database"))
        );
        assert!(dsl::parse("perspective x: X [show: nonsense]\n").is_err());
        // ops, written back through the patcher
        let mut s = crate::Session::new();
        s.set_source(src);
        let r = s.apply(r#"{"ops":[{"op":"perspective","id":"delivery","label":"Squad"},{"op":"perspective","id":"ux","remove":true},{"op":"set","key":"type","value":"product"}]}"#);
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("type: product\n")
                && out.contains(
                    "perspective delivery: Squad [tags: delivery team, show: timeline todo gantt kanban risks table]"
                )
                && !out.contains("perspective ux"),
            "{out}"
        );
        let ids: Vec<String> = s.perspectives().into_iter().map(|p| p.id).collect();
        assert_eq!(
            ids,
            [
                "product",
                "tech",
                "user",
                "legal",
                "business",
                "marketing",
                "design",
                "exec",
                "delivery"
            ]
        );
        assert!(s.apply(r#"{"op":"perspective","id":"x","show":["nope"]}"#)["ok"] == false);
        // from inside a canvas, the board's own things go to the top board
        let mut c = crate::Session::new();
        c.set_source("type: project\npay: P [canvas: inner]\n\ncanvas inner: Inner {\n  x: X\n}\n");
        c.open_canvas(&["inner".into()]).unwrap();
        assert_eq!(c.apply_top(r#"{"ops":[{"op":"perspective","id":"audit","label":"Audit"},{"op":"set","key":"type","value":"product"}]}"#)["ok"], true);
        let out = c.source("dsl").unwrap();
        assert!(
            out.starts_with("type: product\n\nperspective audit: Audit\n"),
            "{out}"
        );
        assert!(c.doc.node("x").is_some(), "still in the canvas");
    }

    #[test]
    fn a_perspective_shows_its_part_of_the_board() {
        let doc = dsl::parse(
            "type: project\n\nperspective exec: Leadership [tags: exec boss, show: adr risks, details: off]\nperspective ux: Design [tags: ux]\n\napi: Orders API [tags: tech, detail: Rust]\nbeta: Open beta [tags: exec, detail: 28 Oct]\nflow: Checkout flow [tags: ux]\napi -> beta\n\nadr db: Use Postgres [accepted] {\n  decision: Postgres\n}\n\ngroup s: Section {\n  inner: Inner [tags: exec]\n  other: Other\n}\ncomment on beta: when?\ncomment on api: why Rust?\n",
        )
        .unwrap();
        let ps = all(&doc);
        let ids: Vec<&str> = ps.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "delivery",
                "product",
                "tech",
                "legal",
                "business",
                "marketing",
                "design",
                "exec",
                "ux"
            ]
        );
        let lead = ps.iter().find(|p| p.id == "exec").unwrap();
        assert_eq!(
            lead.label, "Leadership",
            "the board's own comes after the built-in ones"
        );
        let v = view(&doc, lead);
        let nodes: Vec<&str> = v.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(nodes, ["beta", "db.decision", "inner"]);
        let groups: Vec<&str> = v.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(
            groups,
            ["db", "s"],
            "a section stays around what it still shows"
        );
        assert!(v.edges.is_empty(), "an arrow to something hidden goes");
        assert_eq!(v.comments.len(), 1);
        assert!(v.nodes.iter().all(|n| n.detail.is_none()), "details off");
        // a sketch has none of its own
        assert!(builtins(BoardKind::Sketch).is_empty());
        assert_eq!(builtins(BoardKind::Product)[0].id, "product");
        assert!(builtins(BoardKind::Project)
            .iter()
            .all(|p| purpose(&p.id).is_some()));
    }

    #[test]
    fn a_perspective_starts_high_level_and_opens_in_place() {
        let doc = dsl::parse(
            "type: project\n\nweb: Storefront\ngroup backend: Backend [blue] {\n  api: Orders API\n  db: Postgres\n  group jobs: Jobs {\n    worker: Worker\n  }\n}\nrisks r: Risks {\n  Fraud [p: 5, i: 5]\n  Delay [p: 1, i: 2]\n}\ntimeline t: History {\n  2026-10-28: Beta\n}\nweb -> api: orders\nweb -> db\napi -> worker\ncomment on db: which version?\n",
        )
        .unwrap();
        let p = builtins(BoardKind::Project)
            .into_iter()
            .find(|p| p.id == "delivery")
            .unwrap();
        let v = fold(&doc, &p, &[]);
        // the backend and the risks are cards; the timeline (delivery's headline) stays open
        let card = v.node("@fold/backend").unwrap();
        assert_eq!(card.label, "Backend");
        assert_eq!(
            card.detail.as_deref(),
            Some("2 items, 1 section · Orders API, Postgres, Jobs")
        );
        assert_eq!(card.color.as_deref(), Some("blue"));
        assert_eq!(
            v.node("@fold/r").unwrap().detail.as_deref(),
            Some("2 risks, 1 high · Fraud, Delay")
        );
        assert!(v.group("t").is_some() && v.node("@fold/t").is_none());
        assert!(v.node("api").is_none() && v.group("jobs").is_none());
        // arrows go to the card, once; the one inside it goes
        let edges: Vec<(&str, &str)> = v
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();
        assert_eq!(edges, [("web", "@fold/backend")]);
        assert_eq!(v.comments[0].on.as_deref(), Some("@fold/backend"));
        // opened, it is as it was
        let open = fold(&doc, &p, &["backend".to_string()]);
        assert!(open.node("api").is_some() && open.node("@fold/backend").is_none());
        assert_eq!(
            purpose("business"),
            Some("Money, pricing, deals and the numbers")
        );
    }
}
