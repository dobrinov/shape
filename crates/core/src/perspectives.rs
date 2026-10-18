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
    "sequence", "lanes", "sticky", "cycle",
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

/// The perspectives a board of this type comes with.
pub fn builtins(kind: BoardKind) -> Vec<Perspective> {
    let exec = p(
        "exec",
        "Senior management",
        &["exec"],
        &["adr", "risks", "timeline", "chart"],
        false,
    );
    let tech = p(
        "tech",
        "Technical",
        &["tech"],
        &["entity", "sequence", "lanes", "adr"],
        true,
    );
    let product = p(
        "product",
        "Product",
        &["product"],
        &["mindmap", "kanban", "timeline", "lanes", "chart"],
        true,
    );
    // the business around it, on product and project boards alike
    let business = || {
        vec![
            p(
                "legal",
                "Legal",
                &["legal"],
                &["adr", "risks", "table"],
                true,
            ),
            p(
                "finance",
                "Finance",
                &["finance"],
                &["chart", "table", "gantt"],
                true,
            ),
            p(
                "marketing",
                "Marketing",
                &["marketing"],
                &["timeline", "chart", "mindmap"],
                true,
            ),
            p(
                "commercial",
                "Commercial",
                &["commercial"],
                &["table", "chart", "kanban"],
                true,
            ),
            p(
                "design",
                "Design",
                &["design"],
                &["mindmap", "lanes", "sticky"],
                true,
            ),
        ]
    };
    match kind {
        BoardKind::Sketch => Vec::new(),
        BoardKind::Product => {
            let mut v = vec![
                product,
                tech,
                p(
                    "user",
                    "User",
                    &["user"],
                    &["lanes", "mindmap", "sticky"],
                    true,
                ),
                exec,
            ];
            v.extend(business());
            v
        }
        BoardKind::Project => {
            let mut v = vec![
                exec,
                p(
                    "team",
                    "Team",
                    &["team"],
                    &["kanban", "gantt", "risks", "timeline", "table"],
                    true,
                ),
                product,
                tech,
            ];
            v.extend(business());
            v
        }
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
        let r = s.apply(r#"{"ops":[{"op":"perspective","id":"team","label":"Squad"},{"op":"perspective","id":"ux","remove":true},{"op":"set","key":"type","value":"product"}]}"#);
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("type: product\n")
                && out.contains(
                    "perspective team: Squad [tags: team, show: kanban gantt risks timeline table]"
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
                "exec",
                "legal",
                "finance",
                "marketing",
                "commercial",
                "design",
                "team"
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
                "exec",
                "team",
                "product",
                "tech",
                "legal",
                "finance",
                "marketing",
                "commercial",
                "design",
                "ux"
            ]
        );
        assert_eq!(
            ps[0].label, "Leadership",
            "the board's own overrides the built-in"
        );
        let v = view(&doc, &ps[0]);
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
    }
}
