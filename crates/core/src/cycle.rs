//! Cycles: a group whose nodes are steps that come round again, drawn on a
//! circle with an arrow from each step to the next (the last to the first).
//!
//! ```text
//! cycle loop: Build loop {
//!   Plan
//!   Build [blue]
//!   Measure
//!   Learn [detail: talk to five users]
//! }
//! ```
use crate::charts::{Mark, Paint, Plot};
use crate::layout::Rect;
use crate::model::{Document, Group, Node};
use crate::text;

const SIZE: f64 = 14.0;
const LINE: f64 = 18.0;
const PAD_X: f64 = 16.0;
const PAD_Y: f64 = 10.0;
const MAX_W: f64 = 170.0;

/// The cycle a node is a step of.
pub fn cycle_of<'a>(doc: &'a Document, n: &Node) -> Option<&'a Group> {
    n.group
        .as_deref()
        .and_then(|g| doc.group(g))
        .filter(|g| g.cycle)
}

/// The steps of cycle `g`, in order.
pub fn steps_of<'a>(doc: &'a Document, g: &str) -> Vec<&'a Node> {
    doc.nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g))
        .collect()
}

/// An arrowhead at (x, y) pointing along angle `a` (radians).
fn head(x: f64, y: f64, a: f64) -> String {
    let (l, w) = (10.0, 5.5);
    let (bx, by) = (x - l * a.cos(), y - l * a.sin());
    let (nx, ny) = (-a.sin() * w, a.cos() * w);
    format!(
        "M{:.1} {:.1}L{:.1} {:.1}L{:.1} {:.1}Z",
        x,
        y,
        bx + nx,
        by + ny,
        bx - nx,
        by - ny
    )
}

/// The cycle drawn: its steps evenly round a circle (the first at the top,
/// going clockwise), an arc with an arrowhead from each to the next.
pub fn plot(doc: &Document, g: &Group) -> Plot {
    let steps = steps_of(doc, &g.id);
    let mut p = Plot::default();
    let boxes: Vec<(Vec<String>, f64, f64)> = steps
        .iter()
        .map(|n| {
            let lines: Vec<String> = text::wrap(n.display(), SIZE, MAX_W)
                .into_iter()
                .take(3)
                .collect();
            let w = lines
                .iter()
                .map(|l| text::width(l, SIZE))
                .fold(60.0, f64::max)
                + 2.0 * PAD_X;
            let h = lines.len().max(1) as f64 * LINE + 2.0 * PAD_Y;
            (lines, w, h)
        })
        .collect();
    let n = steps.len();
    if n == 0 {
        p.w = 360.0;
        p.h = 120.0;
        p.marks.push(Mark::Text {
            x: 180.0,
            y: 64.0,
            text: "No steps yet: one per line".into(),
            size: 12.0,
            anchor: "middle",
            paint: Paint::Muted,
            bold: false,
        });
        return p;
    }
    let widest = boxes.iter().map(|b| b.1).fold(0.0, f64::max);
    let tallest = boxes.iter().map(|b| b.2).fold(0.0, f64::max);
    // a radius that keeps neighbouring boxes apart
    let gap = 36.0;
    let around = boxes.iter().map(|b| b.1.max(b.2) + gap).sum::<f64>();
    let r = (around / std::f64::consts::TAU)
        .max(110.0)
        .max(if n == 2 { widest } else { 0.0 });
    let (cx, cy) = (r + widest / 2.0 + 8.0, r + tallest / 2.0 + 8.0);
    p.w = 2.0 * cx;
    p.h = 2.0 * cy;
    let angle =
        |i: usize| -std::f64::consts::FRAC_PI_2 + std::f64::consts::TAU * i as f64 / n as f64;
    let centre: Vec<(f64, f64)> = (0..n)
        .map(|i| (cx + r * angle(i).cos(), cy + r * angle(i).sin()))
        .collect();
    // how far along the circle a box reaches from its centre (so arcs start at its edge)
    let reach = |b: &(Vec<String>, f64, f64), a: f64| -> f64 {
        // the box's half-size in the direction of travel (tangent)
        let (tx, ty) = (-a.sin(), a.cos());
        let half = (b.1 / 2.0 * tx.abs()).max(b.2 / 2.0 * ty.abs()).max(16.0);
        (half + 8.0) / r
    };
    if n > 1 {
        for i in 0..n {
            let j = (i + 1) % n;
            let a0 = angle(i) + reach(&boxes[i], angle(i));
            let mut a1 = angle(j) - reach(&boxes[j], angle(j));
            if j == 0 {
                a1 += std::f64::consts::TAU;
            }
            if a1 <= a0 {
                continue;
            }
            let (x0, y0) = (cx + r * a0.cos(), cy + r * a0.sin());
            let (x1, y1) = (cx + r * a1.cos(), cy + r * a1.sin());
            let large = if a1 - a0 > std::f64::consts::PI { 1 } else { 0 };
            p.marks.push(Mark::Path {
                d: format!("M{x0:.1} {y0:.1}A{r:.1} {r:.1} 0 {large} 1 {x1:.1} {y1:.1}"),
                stroke: Some(Paint::Muted),
                fill: None,
                width: 2.0,
                opacity: 1.0,
            });
            // the head points along the circle at its end
            p.marks.push(Mark::Path {
                d: head(x1, y1, a1 + std::f64::consts::FRAC_PI_2),
                stroke: None,
                fill: Some(Paint::Muted),
                width: 0.0,
                opacity: 1.0,
            });
        }
    }
    for (i, (node, b)) in steps.iter().zip(&boxes).enumerate() {
        let (x, y) = centre[i];
        let rect = Rect {
            x: x - b.1 / 2.0,
            y: y - b.2 / 2.0,
            w: b.1,
            h: b.2,
        };
        let tint = node.color.clone().or_else(|| g.color.clone());
        p.marks.push(Mark::Rect {
            r: Rect {
                x: rect.x - 1.5,
                y: rect.y - 1.5,
                w: rect.w + 3.0,
                h: rect.h + 3.0,
            },
            paint: tint.clone().map(Paint::Color).unwrap_or(Paint::Grid),
            round: 13.5,
        });
        p.marks.push(Mark::Rect {
            r: rect,
            paint: tint.map(Paint::Soft).unwrap_or(Paint::Bg),
            round: 12.0,
        });
        for (k, l) in b.0.iter().enumerate() {
            p.marks.push(Mark::Text {
                x,
                y: rect.y + PAD_Y + k as f64 * LINE + SIZE * 0.95,
                text: l.clone(),
                size: SIZE,
                anchor: "middle",
                paint: Paint::Fg,
                bold: true,
            });
        }
        p.rows.push((node.id.clone(), rect));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl;

    #[test]
    fn a_cycle_round_trips_and_draws_round() {
        let src = "cycle loop: Build loop [blue] {\n  Plan\n  Build [green]\n  Measure\n  Learn [id: learn, detail: talk to users]\n}\n";
        let doc = dsl::parse(src).unwrap();
        let g = doc.group("loop").unwrap();
        assert!(g.cycle);
        let ids: Vec<&str> = steps_of(&doc, "loop")
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(ids, ["loop.plan", "loop.build", "loop.measure", "learn"]);
        assert_eq!(dsl::print(&doc), src);
        assert_eq!(crate::from_json(&crate::to_json(&doc)).unwrap(), doc);
        let p = plot(&doc, g);
        assert_eq!(p.rows.len(), 4);
        let arcs = p
            .marks
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    Mark::Path {
                        stroke: Some(_),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            arcs, 4,
            "an arrow from each step to the next, the last back to the first"
        );
        // the first at the top, the third at the bottom
        let (top, bottom) = (&p.rows[0].1, &p.rows[2].1);
        assert!(
            top.y < bottom.y && (top.x + top.w / 2.0 - (bottom.x + bottom.w / 2.0)).abs() < 1.0
        );
        for (i, (_, a)) in p.rows.iter().enumerate() {
            for (_, b) in &p.rows[i + 1..] {
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
                    "steps overlap: {a:?} {b:?}"
                );
            }
        }
        // ops
        let mut s = crate::Session::new();
        s.set_source(src);
        assert_eq!(s.apply(r#"{"ops":[{"op":"add_group","id":"c2","label":"Loop","cycle":true},{"op":"add_node","id":"c2.a","label":"A","group":"c2"}]}"#)["ok"], true);
        assert!(s
            .source("dsl")
            .unwrap()
            .contains("cycle c2: Loop {\n  A\n}"));
    }
}
