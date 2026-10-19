//! To-do lists: a group whose nodes are items with a checkbox (`done` ticks
//! one), in order, sub-items indented under the item above them. An item
//! may have an owner and a due date (`date`); one past due and not done is
//! drawn in red.
//!
//! ```text
//! todo launch: Launch checklist {
//!   [x] Release notes [owner: ana]
//!   [ ] Rollback tested [due: 2026-10-28]
//!     [ ] On staging
//!   [ ] Tell support
//! }
//! ```
use std::cell::Cell;

use crate::charts::{Mark, Paint, Plot};
use crate::layout::Rect;
use crate::model::{Document, Group, Node};
use crate::text;

const SIZE: f64 = 14.0;
const LINE: f64 = 19.0;
const ROW_PAD: f64 = 7.0;
const BOX: f64 = 16.0;
const INDENT: f64 = 24.0;
const TEXT_W: f64 = 260.0;
const META_GAP: f64 = 14.0;
const HEAD: f64 = 30.0;

thread_local! {
    /// Today (days since 1970) while a board is drawn, for overdue items.
    static TODAY: Cell<Option<i64>> = const { Cell::new(None) };
}

/// Draw with `today` (days since 1970) as the day overdue items are judged by.
pub fn with_today<T>(today: Option<i64>, f: impl FnOnce() -> T) -> T {
    let before = TODAY.with(|t| t.replace(today));
    let out = f();
    TODAY.with(|t| t.set(before));
    out
}

/// The to-do list a node is an item of.
pub fn todo_of<'a>(doc: &'a Document, n: &Node) -> Option<&'a Group> {
    n.group
        .as_deref()
        .and_then(|g| doc.group(g))
        .filter(|g| g.todo)
}

/// The items of list `g`, in order.
pub fn items_of<'a>(doc: &'a Document, g: &str) -> Vec<&'a Node> {
    doc.nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g))
        .collect()
}

pub fn is_done(n: &Node) -> bool {
    n.done.is_some_and(|d| d >= 100.0)
}

/// (done, all) of list `g`.
pub fn progress(doc: &Document, g: &str) -> (usize, usize) {
    let items = items_of(doc, g);
    (items.iter().filter(|n| is_done(n)).count(), items.len())
}

/// An item's due date, as written and as a day (days since 1970).
fn due(n: &Node) -> Option<(String, Option<i64>)> {
    let d = n.date.as_deref()?;
    let day = crate::gantt::parse_date(d).or_else(|| crate::timeline::parse(d).map(|x| x.0));
    Some((d.to_string(), day))
}

/// A due date as people read it: `28 Oct` (with the year when it is not today's).
fn short_date(written: &str, day: Option<i64>) -> String {
    let Some(day) = day else {
        return written.to_string();
    };
    let s = crate::gantt::format_date(day);
    let (y, m, d) = (&s[..4], &s[5..7], s[8..].trim_start_matches('0'));
    let month = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][m.parse::<usize>().unwrap_or(1).clamp(1, 12) - 1];
    let this_year = TODAY
        .with(|t| t.get())
        .map(|t| crate::gantt::format_date(t)[..4].to_string());
    if this_year.as_deref() == Some(y) {
        format!("{d} {month}")
    } else {
        format!("{d} {month} {y}")
    }
}

/// The list drawn: progress at the top (`2/5` and a bar), then each item
/// with its checkbox, text, owner and due date.
pub fn plot(doc: &Document, g: &Group) -> Plot {
    let items = items_of(doc, &g.id);
    let today = TODAY.with(|t| t.get());
    let mut p = Plot::default();
    // what goes right of the text: owner and due date
    let metas: Vec<String> = items
        .iter()
        .map(|n| {
            let mut m = Vec::new();
            if let Some(o) = &n.owner {
                m.push(o.clone());
            }
            if let Some((w, d)) = due(n) {
                m.push(short_date(&w, d));
            }
            m.join(" · ")
        })
        .collect();
    let meta_w = metas
        .iter()
        .map(|m| text::width(m, 12.5))
        .fold(0.0, f64::max);
    let text_w = items
        .iter()
        .map(|n| {
            let lines = text::wrap(n.display(), SIZE, TEXT_W);
            lines
                .iter()
                .map(|l| text::width(l, SIZE))
                .fold(0.0, f64::max)
                + n.level as f64 * INDENT
        })
        .fold(120.0, f64::max);
    let w = BOX + 10.0 + text_w + if meta_w > 0.0 { META_GAP + meta_w } else { 0.0 };
    // progress
    let (done, all) = progress(doc, &g.id);
    p.marks.push(Mark::Text {
        x: 0.0,
        y: 12.0,
        text: if all == 0 {
            "Nothing to do yet".into()
        } else {
            format!("{done} of {all} done")
        },
        size: 12.0,
        anchor: "start",
        paint: Paint::Muted,
        bold: true,
    });
    let bar_y = 20.0;
    p.marks.push(Mark::Rect {
        r: Rect {
            x: 0.0,
            y: bar_y,
            w,
            h: 4.0,
        },
        paint: Paint::Header,
        round: 2.0,
    });
    if all > 0 && done > 0 {
        p.marks.push(Mark::Rect {
            r: Rect {
                x: 0.0,
                y: bar_y,
                w: w * done as f64 / all as f64,
                h: 4.0,
            },
            paint: Paint::Color(g.color.clone().unwrap_or_else(|| "green".into())),
            round: 2.0,
        });
    }
    let mut y = HEAD;
    for (n, meta) in items.iter().zip(&metas) {
        let x = n.level as f64 * INDENT;
        let lines: Vec<String> = text::wrap(n.display(), SIZE, TEXT_W)
            .into_iter()
            .take(4)
            .collect();
        let h = lines.len().max(1) as f64 * LINE + 2.0 * ROW_PAD;
        let ticked = is_done(n);
        let late =
            !ticked && today.is_some_and(|t| due(n).and_then(|(_, d)| d).is_some_and(|d| d < t));
        let paint = n.color.clone().map(Paint::Color);
        // the checkbox
        let by = y + ROW_PAD + (LINE - BOX) / 2.0;
        if ticked {
            p.marks.push(Mark::Rect {
                r: Rect {
                    x,
                    y: by,
                    w: BOX,
                    h: BOX,
                },
                paint: paint.clone().unwrap_or(Paint::Color("green".into())),
                round: 4.0,
            });
            p.marks.push(Mark::Path {
                d: format!("M{} {}l3.5 3.5l6.5 -7", x + 4.0, by + 8.5),
                stroke: Some(Paint::Bg),
                fill: None,
                width: 2.2,
                opacity: 1.0,
            });
        } else {
            p.marks.push(Mark::Rect {
                r: Rect {
                    x,
                    y: by,
                    w: BOX,
                    h: BOX,
                },
                paint: if late {
                    Paint::Color("red".into())
                } else {
                    Paint::Muted
                },
                round: 4.0,
            });
            p.marks.push(Mark::Rect {
                r: Rect {
                    x: x + 1.6,
                    y: by + 1.6,
                    w: BOX - 3.2,
                    h: BOX - 3.2,
                },
                paint: Paint::Bg,
                round: 2.8,
            });
        }
        // the text, struck through when done
        let tx = x + BOX + 10.0;
        for (k, l) in lines.iter().enumerate() {
            let ly = y + ROW_PAD + k as f64 * LINE + SIZE * 0.95;
            p.marks.push(Mark::Text {
                x: tx,
                y: ly,
                text: l.clone(),
                size: SIZE,
                anchor: "start",
                paint: if ticked { Paint::Muted } else { Paint::Fg },
                bold: false,
            });
            if ticked {
                p.marks.push(Mark::Line {
                    x1: tx,
                    y1: ly - SIZE * 0.32,
                    x2: tx + text::width(l, SIZE),
                    y2: ly - SIZE * 0.32,
                    paint: Paint::Muted,
                    width: 1.2,
                });
            }
        }
        if !meta.is_empty() {
            p.marks.push(Mark::Text {
                x: w,
                y: y + ROW_PAD + SIZE * 0.95,
                text: meta.clone(),
                size: 12.5,
                anchor: "end",
                paint: if late {
                    Paint::Color("red".into())
                } else {
                    Paint::Muted
                },
                bold: late,
            });
        }
        p.rows.push((n.id.clone(), Rect { x, y, w: w - x, h }));
        y += h;
    }
    p.w = w;
    p.h = y.max(HEAD + 10.0);
    p
}

/// List `gid`'s items as text, one per line (`[x] Release notes [owner: ana]`).
pub fn text_of(doc: &Document, gid: &str) -> Option<String> {
    let g = doc.group(gid).filter(|g| g.todo)?;
    let mut out = String::new();
    crate::dsl::todo_block(doc, g, "", &mut out);
    let lines: Vec<&str> = out.lines().collect();
    if lines.len() < 2 {
        return Some(String::new());
    }
    Some(
        lines[1..lines.len() - 1]
            .iter()
            .map(|l| l.strip_prefix("  ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The ops that make list `gid` what `text` says: items keep their ids (and
/// arrows) when their line keeps them; new ones are added, missing ones
/// removed, all in order.
pub fn edit_ops(doc: &Document, gid: &str, text: &str) -> Result<Vec<serde_json::Value>, String> {
    use serde_json::json;
    doc.group(gid)
        .filter(|g| g.todo)
        .ok_or_else(|| format!("no to-do list `{gid}`"))?;
    let body: String = text
        .lines()
        .map(|l| format!("  {}\n", l.trim_end()))
        .collect();
    let fresh = crate::dsl::parse(&format!("todo {gid} {{\n{body}}}\n")).map_err(|e| {
        e.iter()
            .map(|d| format!("line {}: {}", d.line.saturating_sub(1), d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let want = items_of(&fresh, gid);
    let mut ops = Vec::new();
    for (i, n) in want.iter().enumerate() {
        let mine = doc
            .node(&n.id)
            .is_some_and(|o| o.group.as_deref() == Some(gid));
        if !mine && doc.id_taken(&n.id) {
            return Err(format!("id `{}` is already used", n.id));
        }
        let mut op = if mine {
            json!({"op": "update_node", "id": n.id})
        } else {
            json!({"op": "add_node", "id": n.id, "group": gid})
        };
        let o = op.as_object_mut().unwrap();
        o.insert("label".into(), json!(n.label));
        o.insert("done".into(), json!(if is_done(n) { 100 } else { 0 }));
        o.insert("level".into(), json!(n.level));
        o.insert("index".into(), json!(i));
        for (k, v) in [
            ("owner", &n.owner),
            ("date", &n.date),
            ("detail", &n.detail),
            ("color", &n.color),
        ] {
            match v {
                Some(v) => {
                    o.insert(k.into(), json!(v));
                }
                None if mine => {
                    o.insert(k.into(), json!(""));
                }
                None => {}
            }
        }
        ops.push(op);
    }
    for n in items_of(doc, gid) {
        if !want.iter().any(|w| w.id == n.id) {
            ops.push(json!({"op": "remove", "id": n.id}));
        }
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;

    const SRC: &str = "todo launch: Launch checklist {\n  [x] Release notes [owner: ana]\n  [ ] Rollback tested [due: 2026-10-28]\n    [ ] On staging\n  [ ] Tell support [id: tell]\n}\n";

    #[test]
    fn a_todo_list_round_trips_and_ticks() {
        let doc = dsl::parse(SRC).unwrap();
        let g = doc.group("launch").unwrap();
        assert!(g.todo);
        let ids: Vec<&str> = items_of(&doc, "launch")
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "launch.release-notes",
                "launch.rollback-tested",
                "launch.on-staging",
                "tell"
            ]
        );
        let staging = doc.node("launch.on-staging").unwrap();
        assert_eq!(staging.level, 1);
        assert_eq!(
            doc.node("launch.rollback-tested").unwrap().date.as_deref(),
            Some("2026-10-28")
        );
        assert_eq!(progress(&doc, "launch"), (1, 4));
        assert_eq!(dsl::print(&doc), SRC);
        assert_eq!(crate::from_json(&crate::to_json(&doc)).unwrap(), doc);
        // Markdown-style items read the same
        let md = dsl::parse("todo t: T {\n- [x] One\n- [ ] Two\n  - [ ] Two a\n}\n").unwrap();
        assert_eq!(progress(&md, "t"), (1, 3));
        assert_eq!(md.node("t.two-a").unwrap().level, 1);
        // drawn: a row per item, sub-items further in, overdue in red
        let p = with_today(crate::gantt::parse_date("2026-11-01"), || plot(&doc, g));
        assert_eq!(p.rows.len(), 4);
        assert!(p.rows[2].1.x > p.rows[1].1.x, "a sub-item is indented");
        assert!(
            p.marks.iter().any(|m| matches!(m, Mark::Text { text, paint: Paint::Color(c), .. } if text == "28 Oct" && c == "red")),
            "an item past its date is red"
        );
        assert!(p
            .marks
            .iter()
            .any(|m| matches!(m, Mark::Text { text, .. } if text == "1 of 4 done")));
        // ticked by an op, written back
        let mut s = crate::Session::new();
        s.set_source(SRC);
        assert_eq!(
            s.apply(r#"{"op":"update_node","id":"tell","done":100}"#)["ok"],
            true
        );
        assert!(s
            .source("dsl")
            .unwrap()
            .contains("  [x] Tell support [id: tell]\n"));
        assert_eq!(
            s.apply(r#"{"op":"update_node","id":"tell","done":0}"#)["ok"],
            true
        );
        assert!(s
            .source("dsl")
            .unwrap()
            .contains("  [ ] Tell support [id: tell]\n"));
        // edited as text: ticked, reordered, one added, one gone
        let t = text_of(&s.doc, "launch").unwrap();
        assert!(t.starts_with("[x] Release notes [owner: ana]\n[ ] Rollback tested [due: 2026-10-28]\n  [ ] On staging"), "{t}");
        let ops = edit_ops(
            &s.doc,
            "launch",
            "[ ] Tell support [id: tell]\n[x] Release notes [owner: sam]\n[ ] Write the blog post",
        )
        .unwrap();
        let r = s.apply(&serde_json::json!({ "ops": ops }).to_string());
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("todo launch: Launch checklist {\n  [ ] Tell support [id: tell]\n  [x] Release notes [owner: sam]\n  [ ] Write the blog post\n}"),
            "{out}"
        );
        // a prompt shows it as a checklist
        let md = crate::prompt::selection(&s.doc, &["launch".to_string()]);
        assert!(md.contains("- [x] Release notes (sam)"), "{md}");
    }
}
