//! Edge geometry: which side of a box an edge leaves from, ports spread
//! along that side, and orthogonal / curved / straight paths between them.
use std::collections::HashMap;

use serde::Serialize;

use crate::layout::{Layout, Rect};
use crate::model::*;
use crate::text::n;

const STUB: f64 = 18.0;
const RADIUS: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn normal(self) -> (f64, f64) {
        match self {
            Side::Left => (-1.0, 0.0),
            Side::Right => (1.0, 0.0),
            Side::Top => (0.0, -1.0),
            Side::Bottom => (0.0, 1.0),
        }
    }
    fn horizontal(self) -> bool {
        matches!(self, Side::Left | Side::Right)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgePath {
    pub id: String,
    /// SVG path data.
    pub d: String,
    /// Where the label (or a comment pin) goes.
    pub mid: (f64, f64),
    /// Arrowhead tip and the direction it points (unit vector).
    #[serde(skip)]
    pub end: ((f64, f64), (f64, f64)),
    #[serde(skip)]
    pub start: ((f64, f64), (f64, f64)),
    /// Points along the path, for placing a label elsewhere on it.
    #[serde(skip)]
    pub pts: Vec<(f64, f64)>,
}

impl EdgePath {
    /// The point `f` (0 to 1) of the way along the path.
    pub fn at(&self, f: f64) -> (f64, f64) {
        let p = &self.pts;
        if p.len() < 2 {
            return self.mid;
        }
        let lens: Vec<f64> = p
            .windows(2)
            .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
            .collect();
        let goal = lens.iter().sum::<f64>() * f;
        let mut acc = 0.0;
        for (k, l) in lens.iter().enumerate() {
            if acc + l >= goal && *l > 0.0 {
                let t = (goal - acc) / l;
                return (
                    p[k].0 + (p[k + 1].0 - p[k].0) * t,
                    p[k].1 + (p[k + 1].1 - p[k].1) * t,
                );
            }
            acc += l;
        }
        p[p.len() - 1]
    }
}

/// Shapes whose outline does not follow their bounding box: edges attach at
/// the middle of a side instead of being spread along it.
fn spreadable(layout: &Layout, id: &str) -> bool {
    match layout.node(id) {
        Some(nb) => matches!(
            nb.shape,
            Shape::Box
                | Shape::Rounded
                | Shape::Pill
                | Shape::Cylinder
                | Shape::Document
                | Shape::Sticky
                | Shape::Text
        ),
        None => true,
    }
}

fn choose_sides(a: &Rect, b: &Rect, dir: Direction) -> (Side, Side) {
    let dx = b.cx() - a.cx();
    let dy = b.cy() - a.cy();
    let gap_x = dx.abs() - (a.w + b.w) / 2.0;
    let gap_y = dy.abs() - (a.h + b.h) / 2.0;
    let flow_h = matches!(dir, Direction::Right | Direction::Left);
    let horizontal = match (gap_x > 8.0, gap_y > 8.0) {
        (true, false) => true,
        (false, true) => false,
        (true, true) => {
            if flow_h {
                gap_x * 1.6 > gap_y
            } else {
                gap_x > gap_y * 1.6
            }
        }
        (false, false) => gap_x > gap_y,
    };
    if horizontal {
        if dx >= 0.0 {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        }
    } else if dy >= 0.0 {
        (Side::Bottom, Side::Top)
    } else {
        (Side::Top, Side::Bottom)
    }
}

fn side_point(r: &Rect, side: Side, t: f64) -> (f64, f64) {
    match side {
        Side::Left => (r.x, r.y + r.h * t),
        Side::Right => (r.right(), r.y + r.h * t),
        Side::Top => (r.x + r.w * t, r.y),
        Side::Bottom => (r.x + r.w * t, r.bottom()),
    }
}

/// Where the line from the centre of `r` towards `p` leaves the shape.
fn clip(r: &Rect, shape: Option<Shape>, p: (f64, f64)) -> (f64, f64) {
    let (cx, cy) = (r.cx(), r.cy());
    let (dx, dy) = (p.0 - cx, p.1 - cy);
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return (cx, cy);
    }
    let (hw, hh) = (r.w / 2.0, r.h / 2.0);
    let t = match shape {
        Some(Shape::Circle) | Some(Shape::Cloud) => {
            1.0 / ((dx / hw).powi(2) + (dy / hh).powi(2)).sqrt()
        }
        Some(Shape::Diamond) => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        _ => (hw / dx.abs()).min(hh / dy.abs()),
    };
    (cx + dx * t, cy + dy * t)
}

pub fn route(doc: &Document, layout: &Layout) -> Vec<EdgePath> {
    let dir = doc.settings.direction;
    let routing = doc.settings.routing;
    struct Plan {
        a: Rect,
        b: Rect,
        sa: Side,
        sb: Side,
        pa: (f64, f64),
        pb: (f64, f64),
    }
    let mut plans: Vec<Option<Plan>> = Vec::new();
    for e in &doc.edges {
        // messages of a sequence diagram are drawn straight across (below)
        if crate::sequence::sequence_of(doc, e).is_some() {
            plans.push(None);
            continue;
        }
        let (Some(a), Some(b)) = (layout.rect(&e.from), layout.rect(&e.to)) else {
            plans.push(None);
            continue;
        };
        let (sa, sb) = choose_sides(&a, &b, dir);
        plans.push(Some(Plan {
            a,
            b,
            sa,
            sb,
            pa: (0.0, 0.0),
            pb: (0.0, 0.0),
        }));
    }

    // spread the ends that share a side, ordered by where the other end is
    let mut slots: HashMap<(String, Side), Vec<(usize, bool, f64)>> = HashMap::new();
    for (i, e) in doc.edges.iter().enumerate() {
        let Some(p) = &plans[i] else { continue };
        if e.from == e.to {
            continue;
        }
        let key_a = if p.sa.horizontal() {
            p.b.cy()
        } else {
            p.b.cx()
        };
        let key_b = if p.sb.horizontal() {
            p.a.cy()
        } else {
            p.a.cx()
        };
        slots
            .entry((e.from.clone(), p.sa))
            .or_default()
            .push((i, true, key_a));
        slots
            .entry((e.to.clone(), p.sb))
            .or_default()
            .push((i, false, key_b));
    }
    for ((id, side), mut ends) in slots {
        ends.sort_by(|x, y| x.2.partial_cmp(&y.2).unwrap().then(x.0.cmp(&y.0)));
        let count = ends.len() as f64;
        let spread = spreadable(layout, &id);
        for (k, (i, is_a, _)) in ends.into_iter().enumerate() {
            let p = plans[i].as_mut().unwrap();
            let r = if is_a { p.a } else { p.b };
            let t = if spread && count > 1.0 {
                // keep ports away from rounded corners
                let span = (count - 1.0) * 18.0;
                let len = if side.horizontal() { r.h } else { r.w };
                let span = span.min(len * 0.7);
                0.5 + (k as f64 / (count - 1.0) - 0.5) * span / len
            } else {
                0.5
            };
            let pt = side_point(&r, side, t);
            if is_a {
                p.pa = pt;
            } else {
                p.pb = pt;
            }
        }
    }
    // shapes that aren't boxes: attach on the outline, not the bounding box
    for (i, e) in doc.edges.iter().enumerate() {
        let Some(p) = plans[i].as_mut() else { continue };
        for (id, is_a) in [(&e.from, true), (&e.to, false)] {
            let Some(nb) = layout.node(id) else { continue };
            if !spreadable(layout, id) {
                let r = nb.rect;
                let side = if is_a { p.sa } else { p.sb };
                let (nx, ny) = side.normal();
                let target = (r.cx() + nx * r.w, r.cy() + ny * r.h);
                let pt = if nb.shape == Shape::Person {
                    side_point(&r, side, 0.5)
                } else {
                    clip(&r, Some(nb.shape), target)
                };
                if is_a {
                    p.pa = pt;
                } else {
                    p.pb = pt;
                }
            }
        }
    }

    // lanes: forward orthogonal edges crossing the same gap get their own
    // middle segment so parallel edges do not overlap
    let mut lane: HashMap<usize, f64> = HashMap::new();
    if routing == Routing::Orthogonal {
        let mut gaps: HashMap<(bool, i64, i64), Vec<usize>> = HashMap::new();
        for (i, p) in plans.iter().enumerate() {
            let Some(p) = p else { continue };
            if doc.edges[i].from == doc.edges[i].to {
                continue;
            }
            let h = p.sa.horizontal();
            let (a0, b0) = if h {
                (p.pa.0, p.pb.0)
            } else {
                (p.pa.1, p.pb.1)
            };
            let (ca, cb) = if h {
                (p.pa.1, p.pb.1)
            } else {
                (p.pa.0, p.pb.0)
            };
            if (ca - cb).abs() < 1.0 {
                continue;
            }
            let lo = (a0.min(b0) / 8.0).round() as i64;
            let hi = (a0.max(b0) / 8.0).round() as i64;
            gaps.entry((h, lo, hi)).or_default().push(i);
        }
        for ((h, _, _), mut ids) in gaps {
            if ids.len() < 2 {
                continue;
            }
            let key = |i: &usize| {
                let p = plans[*i].as_ref().unwrap();
                if h {
                    (p.pa.1 + p.pb.1) / 2.0
                } else {
                    (p.pa.0 + p.pb.0) / 2.0
                }
            };
            ids.sort_by(|x, y| key(x).partial_cmp(&key(y)).unwrap());
            let count = ids.len() as f64;
            for (k, i) in ids.into_iter().enumerate() {
                let p = plans[i].as_ref().unwrap();
                let (a0, b0) = if h {
                    (p.pa.0, p.pb.0)
                } else {
                    (p.pa.1, p.pb.1)
                };
                let (lo, hi) = (a0.min(b0) + STUB, a0.max(b0) - STUB);
                if hi > lo {
                    lane.insert(i, lo + (hi - lo) * (k as f64 + 1.0) / (count + 1.0));
                }
            }
        }
    }

    // middle segments that would run closer than GAP to each other (from
    // different gaps, so not spread above) move apart, within their gap
    if routing == Routing::Orthogonal {
        const GAP: f64 = 24.0;
        // (edge, vertical segment?, where, its range, the span it covers)
        let mut segs: Vec<(usize, bool, f64, (f64, f64), (f64, f64))> = Vec::new();
        for (i, p) in plans.iter().enumerate() {
            let Some(p) = p else { continue };
            if doc.edges[i].from == doc.edges[i].to {
                continue;
            }
            let h = p.sa.horizontal();
            let sw = |q: (f64, f64)| if h { q } else { (q.1, q.0) };
            let (p1, p2) = (sw(p.pa), sw(p.pb));
            let (na, nb) = (sw(p.sa.normal()).0, sw(p.sb.normal()).0);
            let forward = (p2.0 - p1.0) * na > 2.0 * STUB - 1.0 && na == -nb;
            if !forward || (p1.1 - p2.1).abs() < 1.0 {
                continue;
            }
            let (lo, hi) = (p1.0.min(p2.0) + STUB, p1.0.max(p2.0) - STUB);
            let at = lane.get(&i).copied().unwrap_or((p1.0 + p2.0) / 2.0);
            segs.push((i, h, at, (lo, hi), (p1.1.min(p2.1), p1.1.max(p2.1))));
        }
        // spreading one group can bring it close to another: a few rounds
        for _ in 0..6 {
            let mut moved = false;
            for s in segs.iter_mut() {
                if let Some(&at) = lane.get(&s.0) {
                    s.2 = at;
                }
            }
            for h in [true, false] {
                let mut list: Vec<_> = segs.iter().filter(|s| s.1 == h).cloned().collect();
                list.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap().then(a.0.cmp(&b.0)));
                // clusters of segments that are close and side by side
                let mut used = vec![false; list.len()];
                for k in 0..list.len() {
                    if used[k] {
                        continue;
                    }
                    let mut cluster = vec![k];
                    used[k] = true;
                    let mut grew = true;
                    while grew {
                        grew = false;
                        for j in 0..list.len() {
                            if used[j] {
                                continue;
                            }
                            let near = cluster.iter().any(|&c| {
                                let (a, b) = (&list[c], &list[j]);
                                (a.2 - b.2).abs() < GAP && a.4 .0 < b.4 .1 && b.4 .0 < a.4 .1
                            });
                            if near {
                                cluster.push(j);
                                used[j] = true;
                                grew = true;
                            }
                        }
                    }
                    if cluster.len() < 2 {
                        continue;
                    }
                    cluster.sort_by(|a, b| list[*a].2.partial_cmp(&list[*b].2).unwrap());
                    let centre =
                        cluster.iter().map(|&c| list[c].2).sum::<f64>() / cluster.len() as f64;
                    let half = (cluster.len() as f64 - 1.0) / 2.0;
                    for (n, &c) in cluster.iter().enumerate() {
                        let (lo, hi) = list[c].3;
                        let want = centre + (n as f64 - half) * GAP;
                        let at = if hi > lo {
                            want.clamp(lo, hi)
                        } else {
                            list[c].2
                        };
                        moved |= (at - list[c].2).abs() > 0.5;
                        lane.insert(list[c].0, at);
                    }
                }
            }
            if !moved {
                break;
            }
        }
    }

    let mut out = Vec::new();
    for (i, e) in doc.edges.iter().enumerate() {
        if let Some(pts) = crate::sequence::message_points(doc, layout, e) {
            let mut path = polyline(&e.id, &pts);
            // the label sits on the message, just above its line
            path.mid = if e.from == e.to {
                let w = e
                    .label
                    .as_deref()
                    .map_or(0.0, |l| crate::text::width(l, 12.0));
                (pts[1].0 + 14.0 + w / 2.0, pts[1].1 + 9.0)
            } else {
                ((pts[0].0 + pts[1].0) / 2.0, pts[0].1 - 12.0)
            };
            out.push(path);
            continue;
        }
        let Some(p) = &plans[i] else { continue };
        if e.from == e.to {
            out.push(self_loop(&e.id, &p.a));
            continue;
        }
        let path = match routing {
            Routing::Straight => {
                let sa = layout.node(&e.from).map(|n| n.shape);
                let sb = layout.node(&e.to).map(|n| n.shape);
                let pa = clip(&p.a, sa, (p.b.cx(), p.b.cy()));
                let pb = clip(&p.b, sb, (p.a.cx(), p.a.cy()));
                polyline(&e.id, &[pa, pb])
            }
            Routing::Curved => curve(&e.id, p.pa, p.sa, p.pb, p.sb),
            Routing::Orthogonal => {
                let mut pts = orthogonal(p.pa, p.sa, p.pb, p.sb, &p.a, &p.b, lane.get(&i).copied());
                // a path across a box goes round the boxes instead
                let boxes = obstacles(layout, &e.from, &e.to, &p.a, &p.b);
                if crosses(&pts, &boxes) {
                    if let Some(better) = around(p.pa, p.sa, p.pb, p.sb, &boxes) {
                        pts = better;
                    }
                }
                polyline(&e.id, &pts)
            }
        };
        out.push(path);
    }
    out
}

/// How far a path keeps from a box it goes round.
const CLEAR: f64 = 12.0;

/// The boxes an arrow from `from` to `to` must not cross, grown by
/// [`CLEAR`]: every node but its two ends (and what is inside them).
fn obstacles(layout: &Layout, from: &str, to: &str, a: &Rect, b: &Rect) -> Vec<Rect> {
    let inside = |r: &Rect, o: &Rect| {
        r.x >= o.x - 0.5
            && r.y >= o.y - 0.5
            && r.right() <= o.right() + 0.5
            && r.bottom() <= o.bottom() + 0.5
    };
    layout
        .nodes
        .iter()
        .filter(|n| n.id != from && n.id != to)
        .map(|n| n.rect)
        .filter(|r| !inside(r, a) && !inside(r, b))
        .map(|r| Rect {
            x: r.x - CLEAR,
            y: r.y - CLEAR,
            w: r.w + 2.0 * CLEAR,
            h: r.h + 2.0 * CLEAR,
        })
        .collect()
}

/// Whether the polyline runs through one of `boxes` (touching its edge is fine).
fn crosses(pts: &[(f64, f64)], boxes: &[Rect]) -> bool {
    pts.windows(2).any(|w| {
        let (x0, x1) = (w[0].0.min(w[1].0), w[0].0.max(w[1].0));
        let (y0, y1) = (w[0].1.min(w[1].1), w[0].1.max(w[1].1));
        boxes.iter().any(|r| {
            // the box grown by CLEAR, less a little, so a path along it does not count
            let (l, t, rr, bb) = (
                r.x + CLEAR * 0.5,
                r.y + CLEAR * 0.5,
                r.right() - CLEAR * 0.5,
                r.bottom() - CLEAR * 0.5,
            );
            x1 > l && x0 < rr && y1 > t && y0 < bb
        })
    })
}

/// An orthogonal path from `pa` (leaving by side `sa`) to `pb` (arriving by
/// `sb`) that goes round `boxes`: the shortest on a grid of the boxes' edges,
/// with few bends. None when there is none.
fn around(
    pa: (f64, f64),
    sa: Side,
    pb: (f64, f64),
    sb: Side,
    boxes: &[Rect],
) -> Option<Vec<(f64, f64)>> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let (na, nb) = (sa.normal(), sb.normal());
    let s = (pa.0 + na.0 * STUB, pa.1 + na.1 * STUB);
    let t = (pb.0 + nb.0 * STUB, pb.1 + nb.1 * STUB);
    // only the boxes near the two ends matter
    let (lx, hx) = (s.0.min(t.0) - 400.0, s.0.max(t.0) + 400.0);
    let (ly, hy) = (s.1.min(t.1) - 400.0, s.1.max(t.1) + 400.0);
    let near: Vec<&Rect> = boxes
        .iter()
        .filter(|r| r.right() > lx && r.x < hx && r.bottom() > ly && r.y < hy)
        .filter(|r| !in_rect(r, s) && !in_rect(r, t))
        .collect();
    let mut xs = vec![s.0, t.0];
    let mut ys = vec![s.1, t.1];
    for r in &near {
        xs.extend([r.x, r.right()]);
        ys.extend([r.y, r.bottom()]);
    }
    for v in [&mut xs, &mut ys] {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    }
    let (nx, ny) = (xs.len(), ys.len());
    let free = |x: f64, y: f64| {
        !near
            .iter()
            .any(|r| x > r.x + 0.5 && x < r.right() - 0.5 && y > r.y + 0.5 && y < r.bottom() - 0.5)
    };
    let find = |v: &[f64], x: f64| v.iter().position(|c| (c - x).abs() < 0.5);
    let (si, sj) = (find(&xs, s.0)?, find(&ys, s.1)?);
    let (ti, tj) = (find(&xs, t.0)?, find(&ys, t.1)?);
    // the first move goes on away from the box, the last comes in towards it
    let dir_of = |d: (f64, f64)| -> usize {
        if d.0 > 0.0 {
            0
        } else if d.0 < 0.0 {
            1
        } else if d.1 > 0.0 {
            2
        } else {
            3
        }
    };
    let start_dir = dir_of(na);
    let end_dir = dir_of((-nb.0, -nb.1));
    const BEND: f64 = 40.0;
    let idx = |i: usize, j: usize, d: usize| (j * nx + i) * 4 + d;
    let mut best = vec![f64::INFINITY; nx * ny * 4];
    let mut from: Vec<Option<usize>> = vec![None; nx * ny * 4];
    let mut heap = BinaryHeap::new();
    let key = |c: f64| Reverse((c * 100.0) as i64);
    best[idx(si, sj, start_dir)] = 0.0;
    heap.push((key(0.0), idx(si, sj, start_dir)));
    let mut goal = None;
    while let Some((Reverse(c), state)) = heap.pop() {
        let cost = c as f64 / 100.0;
        if cost > best[state] + 0.01 {
            continue;
        }
        let d = state % 4;
        let cell = state / 4;
        let (i, j) = (cell % nx, cell / nx);
        if i == ti && j == tj {
            // arriving straight in counts no bend; otherwise one more
            let total = cost + if d == end_dir { 0.0 } else { BEND };
            if goal.is_none_or(|(g, _)| total < g) {
                goal = Some((total, state));
            }
            continue;
        }
        for nd in 0..4 {
            // never straight back
            if (d ^ 1) == nd && d / 2 == nd / 2 {
                continue;
            }
            let (ni, nj) = match nd {
                0 if i + 1 < nx => (i + 1, j),
                1 if i > 0 => (i - 1, j),
                2 if j + 1 < ny => (i, j + 1),
                3 if j > 0 => (i, j - 1),
                _ => continue,
            };
            let (x0, y0, x1, y1) = (xs[i], ys[j], xs[ni], ys[nj]);
            if !free((x0 + x1) / 2.0, (y0 + y1) / 2.0) || !free(x1, y1) {
                continue;
            }
            let step = (x1 - x0).abs() + (y1 - y0).abs() + if nd == d { 0.0 } else { BEND };
            let next = idx(ni, nj, nd);
            if cost + step < best[next] - 0.01 {
                best[next] = cost + step;
                from[next] = Some(state);
                heap.push((key(cost + step), next));
            }
        }
    }
    let (_, mut state) = goal?;
    let mut cells = vec![state / 4];
    while let Some(prev) = from[state] {
        state = prev;
        cells.push(state / 4);
    }
    cells.reverse();
    let mut pts = vec![pa];
    for c in cells {
        let q = (xs[c % nx], ys[c / nx]);
        // keep only the corners
        let n = pts.len();
        if n >= 2 {
            let (a, b) = (pts[n - 2], pts[n - 1]);
            let same_line = ((a.0 - b.0).abs() < 0.5 && (b.0 - q.0).abs() < 0.5)
                || ((a.1 - b.1).abs() < 0.5 && (b.1 - q.1).abs() < 0.5);
            if same_line {
                pts[n - 1] = q;
                continue;
            }
        }
        pts.push(q);
    }
    pts.push(pb);
    Some(pts)
}

fn in_rect(r: &Rect, p: (f64, f64)) -> bool {
    p.0 > r.x && p.0 < r.right() && p.1 > r.y && p.1 < r.bottom()
}

fn orthogonal(
    pa: (f64, f64),
    sa: Side,
    pb: (f64, f64),
    sb: Side,
    a: &Rect,
    b: &Rect,
    lane: Option<f64>,
) -> Vec<(f64, f64)> {
    // work in "horizontal" coordinates and swap back for vertical sides
    let h = sa.horizontal();
    let sw = |p: (f64, f64)| if h { p } else { (p.1, p.0) };
    let (p1, p2) = (sw(pa), sw(pb));
    let (na, nb) = (sw(sa.normal()).0, sw(sb.normal()).0);
    let (ra, rb) = if h {
        (*a, *b)
    } else {
        (
            Rect {
                x: a.y,
                y: a.x,
                w: a.h,
                h: a.w,
            },
            Rect {
                x: b.y,
                y: b.x,
                w: b.h,
                h: b.w,
            },
        )
    };
    let forward = (p2.0 - p1.0) * na > 2.0 * STUB - 1.0 && na == -nb;
    let pts: Vec<(f64, f64)> = if forward {
        if (p1.1 - p2.1).abs() < 1.0 {
            vec![p1, (p2.0, p1.1)]
        } else {
            let mx = lane.unwrap_or((p1.0 + p2.0) / 2.0);
            vec![p1, (mx, p1.1), (mx, p2.1), p2]
        }
    } else {
        // around: leave, go past both boxes on the shorter side, come back
        let o1 = p1.0 + na * STUB;
        let o2 = p2.0 + nb * STUB;
        let above = ra.y.min(rb.y) - STUB;
        let below = ra.bottom().max(rb.bottom()) + STUB;
        let my = if (p1.1 - above).abs() + (p2.1 - above).abs()
            < (p1.1 - below).abs() + (p2.1 - below).abs()
        {
            above
        } else {
            below
        };
        vec![p1, (o1, p1.1), (o1, my), (o2, my), (o2, p2.1), p2]
    };
    pts.into_iter().map(sw).collect()
}

fn unit(from: (f64, f64), to: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let l = (dx * dx + dy * dy).sqrt().max(1e-9);
    (dx / l, dy / l)
}

fn polyline(id: &str, pts: &[(f64, f64)]) -> EdgePath {
    // drop zero-length segments, which would break the rounded corners
    let mut p: Vec<(f64, f64)> = Vec::new();
    for &q in pts {
        if p.last()
            .is_none_or(|l: &(f64, f64)| (l.0 - q.0).abs() > 0.5 || (l.1 - q.1).abs() > 0.5)
        {
            p.push(q);
        }
    }
    if p.len() == 1 {
        p.push(p[0]);
    }
    let mut d = format!("M{},{}", n(p[0].0), n(p[0].1));
    for i in 1..p.len() {
        if i + 1 < p.len() {
            let (prev, cur, next) = (p[i - 1], p[i], p[i + 1]);
            let l1 = ((cur.0 - prev.0).powi(2) + (cur.1 - prev.1).powi(2)).sqrt();
            let l2 = ((next.0 - cur.0).powi(2) + (next.1 - cur.1).powi(2)).sqrt();
            let r = RADIUS.min(l1 / 2.0).min(l2 / 2.0);
            let u1 = unit(prev, cur);
            let u2 = unit(cur, next);
            let a = (cur.0 - u1.0 * r, cur.1 - u1.1 * r);
            let b = (cur.0 + u2.0 * r, cur.1 + u2.1 * r);
            d.push_str(&format!(
                " L{},{} Q{},{} {},{}",
                n(a.0),
                n(a.1),
                n(cur.0),
                n(cur.1),
                n(b.0),
                n(b.1)
            ));
        } else {
            d.push_str(&format!(" L{},{}", n(p[i].0), n(p[i].1)));
        }
    }
    // label at half the length
    let lens: Vec<f64> = p
        .windows(2)
        .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
        .collect();
    let total: f64 = lens.iter().sum();
    let mut acc = 0.0;
    let mut mid = p[0];
    for (k, l) in lens.iter().enumerate() {
        if acc + l >= total / 2.0 && *l > 0.0 {
            let t = (total / 2.0 - acc) / l;
            mid = (
                p[k].0 + (p[k + 1].0 - p[k].0) * t,
                p[k].1 + (p[k + 1].1 - p[k].1) * t,
            );
            break;
        }
        acc += l;
    }
    let last = p.len() - 1;
    EdgePath {
        id: id.to_string(),
        d,
        mid,
        end: (p[last], unit(p[last - 1], p[last])),
        start: (p[0], unit(p[1], p[0])),
        pts: p,
    }
}

fn curve(id: &str, pa: (f64, f64), sa: Side, pb: (f64, f64), sb: Side) -> EdgePath {
    let dist = ((pb.0 - pa.0).powi(2) + (pb.1 - pa.1).powi(2)).sqrt();
    let k = (dist * 0.45).max(30.0);
    let (na, nb) = (sa.normal(), sb.normal());
    let c1 = (pa.0 + na.0 * k, pa.1 + na.1 * k);
    let c2 = (pb.0 + nb.0 * k, pb.1 + nb.1 * k);
    let d = format!(
        "M{},{} C{},{} {},{} {},{}",
        n(pa.0),
        n(pa.1),
        n(c1.0),
        n(c1.1),
        n(c2.0),
        n(c2.1),
        n(pb.0),
        n(pb.1)
    );
    let bez = |t: f64| {
        let u = 1.0 - t;
        let x =
            u * u * u * pa.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * pb.0;
        let y =
            u * u * u * pa.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * pb.1;
        (x, y)
    };
    EdgePath {
        id: id.to_string(),
        d,
        mid: bez(0.5),
        end: (pb, unit(c2, pb)),
        start: (pa, unit(c1, pa)),
        pts: (0..=16).map(|i| bez(i as f64 / 16.0)).collect(),
    }
}

fn self_loop(id: &str, r: &Rect) -> EdgePath {
    let s = (r.right() - 22.0, r.y);
    let e = (r.right(), r.y + 22.0);
    let d = format!(
        "M{},{} C{},{} {},{} {},{}",
        n(s.0),
        n(s.1),
        n(s.0),
        n(s.1 - 44.0),
        n(e.0 + 44.0),
        n(e.1),
        n(e.0),
        n(e.1)
    );
    EdgePath {
        id: id.to_string(),
        d,
        mid: (r.right() + 14.0, r.y - 14.0),
        end: (e, (-1.0, 0.0)),
        start: (s, (0.0, 1.0)),
        pts: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dsl, layout::layout};

    #[test]
    fn arrows_go_round_boxes_in_their_way() {
        // a pinned box right between the two ends of an arrow
        let d = dsl::parse("a: Start [x: 0, y: 100]\nwall: In the way [x: 200, y: 60, w: 120, h: 120]\nb: End [x: 420, y: 100]\na -> b\n").unwrap();
        let l = layout(&d);
        let paths = route(&d, &l);
        let p = paths.iter().find(|p| p.id == "a->b").unwrap();
        let wall = l.rect("wall").unwrap();
        for w in p.pts.windows(2) {
            let (x0, x1) = (w[0].0.min(w[1].0), w[0].0.max(w[1].0));
            let (y0, y1) = (w[0].1.min(w[1].1), w[0].1.max(w[1].1));
            assert!(
                !(x1 > wall.x && x0 < wall.right() && y1 > wall.y && y0 < wall.bottom()),
                "{:?} crosses {wall:?}",
                p.pts
            );
        }
        assert!(p.pts.len() > 2, "it bends round the box: {:?}", p.pts);
        // it still starts and ends where it did, at the two boxes
        let (a, b) = (l.rect("a").unwrap(), l.rect("b").unwrap());
        assert!(
            (p.pts[0].0 - a.right()).abs() < 1.0 && (p.pts.last().unwrap().0 - b.x).abs() < 1.0,
            "{:?}",
            p.pts
        );
        // an arrow with nothing in its way keeps its plain path
        let d = dsl::parse("a: A [x: 0, y: 0]\nb: B [x: 300, y: 0]\na -> b\n").unwrap();
        let l = layout(&d);
        assert_eq!(route(&d, &l)[0].pts.len(), 2);
    }

    #[test]
    fn routes_every_edge() {
        for routing in ["orthogonal", "curved", "straight"] {
            let d = dsl::parse(&format!(
                "routing: {routing}\na -> b\na -> c\nb -> a\nc -> c\ngroup g {{\n x\n}}\na -> g"
            ))
            .unwrap();
            let l = layout(&d);
            let paths = route(&d, &l);
            assert_eq!(paths.len(), 5);
            for p in &paths {
                assert!(
                    p.d.starts_with('M') && !p.d.contains("NaN"),
                    "{routing}: {p:?}"
                );
            }
        }
    }

    #[test]
    fn straight_forward_edge_is_a_line() {
        let d = dsl::parse("a -> b").unwrap();
        let l = layout(&d);
        let p = &route(&d, &l)[0];
        assert_eq!(p.d.matches('L').count(), 1, "{}", p.d);
        assert_eq!(p.end.1, (1.0, 0.0));
    }
}
