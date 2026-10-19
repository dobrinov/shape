//! Timelines: a group whose nodes are events on a line of time.
//!
//! ```text
//! timeline history: Field kit {
//!   Dec 2024: Supplier: no markup on parts
//!   18 Jun 2025: Store listing priced at £5k per region
//!   7–14 May 2026: Regulator approves the design
//!   2026-10-28: Open beta [important, red]
//! }
//! ```
//!
//! An event's `date` is kept as written: a day (`2026-10-28`, `18 Jun
//! 2025`, `Jun 18, 2025`), a month (`Dec 2024`, `2024-12`), a quarter
//! (`Q3 2026`), a year (`2026`), or a span of them (`7–14 May 2026`,
//! `2026-05-07..2026-05-14`, `Jan – Mar 2026`). [`plot`] places the events
//! by date on a line, their labels above and below it, out of each other's
//! way; a span is a bar, an `important` event a star.
use crate::charts::{Mark, Paint, Plot};
use crate::gantt::{civil, parse_date};
use crate::layout::Rect;
use crate::model::{Document, Group, Node};
use crate::text;

const SIZE: f64 = 13.0;
const DATE_SIZE: f64 = 11.0;
const LINE: f64 = 17.0;
/// The widest an event's label gets before it wraps.
const LABEL_W: f64 = 170.0;
const PAD: f64 = 8.0;
/// The least room an event gets on the line after the one before it.
const MIN_GAP: f64 = 64.0;
/// The most room a quiet stretch between two events takes.
const MAX_GAP: f64 = 220.0;
/// Space between the line and the nearest labels.
const GAP: f64 = 22.0;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The timeline a node is an event of.
pub fn timeline_of<'a>(doc: &'a Document, n: &Node) -> Option<&'a Group> {
    n.group
        .as_deref()
        .and_then(|g| doc.group(g))
        .filter(|g| g.timeline)
}

/// The events of timeline `g`, in document order.
pub fn events_of<'a>(doc: &'a Document, g: &str) -> Vec<&'a Node> {
    doc.nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g))
        .collect()
}

fn month(word: &str) -> Option<i64> {
    let w = word.trim_end_matches('.').to_lowercase();
    if w.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| w.starts_with(m) && m.starts_with(&w[..3]))
        .map(|i| i as i64 + 1)
}

fn day(y: i64, m: i64, d: i64) -> Option<i64> {
    parse_date(&format!("{y:04}-{m:02}-{d:02}"))
}

fn month_span(y: i64, m: i64) -> Option<(i64, i64)> {
    let first = day(y, m, 1)?;
    let next = if m == 12 {
        day(y + 1, 1, 1)?
    } else {
        day(y, m + 1, 1)?
    };
    Some((first, next - 1))
}

fn year(s: &str) -> Option<i64> {
    (s.len() == 4)
        .then(|| s.parse::<i64>().ok())
        .flatten()
        .filter(|y| (1000..=9999).contains(y))
}

/// One date (not a span): the first and last day it covers. `ctx` fills in
/// the year and month a short date leaves out (`7` or `7 May` in `7–14 May
/// 2026`).
fn one(s: &str, ctx: Option<(i64, i64)>) -> Option<(i64, i64)> {
    let s = s.trim().trim_end_matches(',');
    // ISO: 2026, 2026-10, 2026-10-28
    let iso: Vec<&str> = s.split('-').collect();
    if iso
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    {
        match iso.as_slice() {
            [y] if year(y).is_some() => {
                let y = year(y)?;
                return Some((day(y, 1, 1)?, day(y, 12, 31)?));
            }
            [y, m] => return month_span(year(y)?, m.parse().ok()?),
            [y, m, d] => {
                let d = day(year(y)?, m.parse().ok()?, d.parse().ok()?)?;
                return Some((d, d));
            }
            _ => {}
        }
    }
    let words: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();
    let num = |w: &str| {
        w.trim_end_matches(['.', 's', 't', 'h', 'n', 'd', 'r'])
            .parse::<i64>()
            .ok()
    };
    match words.as_slice() {
        // Q3 2026
        [q, y] if q.len() == 2 && q.to_lowercase().starts_with('q') => {
            let q: i64 = q[1..].parse().ok().filter(|q| (1..=4).contains(q))?;
            let y = year(y)?;
            Some((month_span(y, q * 3 - 2)?.0, month_span(y, q * 3)?.1))
        }
        // Dec 2024
        [m, y] if month(m).is_some() && year(y).is_some() => month_span(year(y)?, month(m)?),
        // 18 Jun 2025, 18th June 2025
        [d, m, y] if month(m).is_some() && year(y).is_some() => {
            let d = day(year(y)?, month(m)?, num(d)?)?;
            Some((d, d))
        }
        // Jun 18 2025 (Jun 18, 2025)
        [m, d, y] if month(m).is_some() && year(y).is_some() => {
            let d = day(year(y)?, month(m)?, num(d)?)?;
            Some((d, d))
        }
        // the start of a span: 7 May, May 7, May, 7
        [d, m] if month(m).is_some() => {
            let d = day(ctx?.0, month(m)?, num(d)?)?;
            Some((d, d))
        }
        [m, d] if month(m).is_some() && num(d).is_some() => {
            let d = day(ctx?.0, month(m)?, num(d)?)?;
            Some((d, d))
        }
        [m] if month(m).is_some() => month_span(ctx?.0, month(m)?),
        [d] if num(d).is_some() => {
            let (y, m) = ctx?;
            let d = day(y, m, num(d)?)?;
            Some((d, d))
        }
        _ => None,
    }
}

/// A date as people write it: the first and last day it covers (days since
/// 1970-01-01). `None` when it cannot be read.
pub fn parse(s: &str) -> Option<(i64, i64)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // about a date: ~6 Aug 2026, c. 2025, around Q3 2026
    let lower = s.to_lowercase();
    for word in [
        "~", "c. ", "ca. ", "circa ", "about ", "around ", "approx. ",
    ] {
        if lower.starts_with(word) {
            return parse(&s[word.len()..]);
        }
    }
    // early / mid / late in a month: its first, middle or last ten days
    for (word, part) in [("early ", 0), ("mid ", 1), ("mid-", 1), ("late ", 2)] {
        if let Some(rest) = lower.strip_prefix(word) {
            let (a, b) = one(&s[s.len() - rest.len()..], None)?;
            if b - a < 27 || b - a > 30 {
                return None; // a month only
            }
            return Some(match part {
                0 => (a, a + 9),
                1 => (a + 10, a + 19),
                _ => (a + 20, b),
            });
        }
    }
    for sep in ["..", "–", "—", " - ", " to "] {
        if let Some((a, b)) = s.split_once(sep) {
            let end = one(b, None)?;
            let (y, m, _) = civil(end.0);
            let start = one(a, Some((y, m)))?;
            return (start.0 <= end.1).then_some((start.0, end.1));
        }
    }
    one(s, None)
}

/// Whether a date is written as a span (`7–14 May 2026`), drawn as a bar;
/// a month or a year alone is a point whose day is not known.
pub fn is_span(s: &str) -> bool {
    ["..", "–", "—", " - ", " to "]
        .iter()
        .any(|sep| s.contains(sep))
        && parse(s).is_some()
}

/// Why a date cannot be read, for error messages.
pub fn date_error(s: &str) -> String {
    format!("cannot read the date `{s}` (e.g. 2026-10-28, 28 Oct 2026, Oct 2026, mid Oct 2026, Q4 2026, 2026, or a span: 7–14 May 2026)")
}

/// An event on hover: `28 Oct 2026 — Open beta`.
pub fn tooltip(n: &Node) -> String {
    let mut s = match &n.date {
        Some(d) => format!("{d} — {}", n.display()),
        None => n.display().to_string(),
    };
    if n.important {
        s.push_str(" (important)");
    }
    if let Some(d) = &n.detail {
        s.push_str(&format!("\n{d}"));
    }
    s
}

/// A five-pointed star around (cx, cy).
fn star(cx: f64, cy: f64, r: f64) -> String {
    let mut d = String::new();
    for k in 0..10 {
        let a = std::f64::consts::PI / 5.0 * k as f64 - std::f64::consts::FRAC_PI_2;
        let rr = if k % 2 == 0 { r } else { r * 0.45 };
        let (x, y) = (cx + rr * a.cos(), cy + rr * a.sin());
        d.push_str(&format!(
            "{}{:.1} {:.1}",
            if k == 0 { "M" } else { "L" },
            x,
            y
        ));
    }
    d.push('Z');
    d
}

/// Ticks along the line: years over long spans, else months.
fn ticks(t0: i64, t1: i64) -> Vec<(i64, String)> {
    let (y0, m0, _) = civil(t0);
    let (y1, m1, _) = civil(t1);
    let months = (y1 - y0) * 12 + (m1 - m0);
    let mut out = Vec::new();
    if months > 30 {
        let step = ((y1 - y0) / 10 + 1).max(1);
        let mut y = y0 + 1;
        while y <= y1 {
            if let Some(d) = day(y, 1, 1) {
                out.push((d, y.to_string()));
            }
            y += step;
        }
    } else {
        let step = (months / 12 + 1).max(1);
        let (mut y, mut m) = (y0, m0);
        let mut k = 0;
        loop {
            m += 1;
            if m > 12 {
                m = 1;
                y += 1;
            }
            if y > y1 || (y == y1 && m > m1) {
                break;
            }
            k += 1;
            if k % step != 0 {
                continue;
            }
            if let Some(d) = day(y, m, 1) {
                let name = MONTH_NAMES[(m - 1) as usize];
                // the year on the first tick of each year
                let first = out
                    .last()
                    .is_none_or(|(prev, _): &(i64, String)| civil(*prev).0 != y);
                out.push((
                    d,
                    if first {
                        format!("{name} {y}")
                    } else {
                        name.to_string()
                    },
                ));
            }
        }
    }
    out
}

struct Placed<'a> {
    node: &'a Node,
    span: (i64, i64),
    bar: bool,
    lines: Vec<String>,
    w: f64,
    h: f64,
}

/// The timeline drawn: a line with ticks, each event a dot (a star when it
/// is important, a bar when it spans days) and its date and label in a
/// lane above or below, connected to its place on the line.
pub fn plot(doc: &Document, g: &Group) -> Plot {
    let mut events: Vec<Placed> = events_of(doc, &g.id)
        .into_iter()
        .filter_map(|n| {
            let span = parse(n.date.as_deref()?)?;
            let mut lines = text::wrap(n.display(), SIZE, LABEL_W);
            // three lines at most; the hover card has the rest
            if lines.len() > 3 {
                lines.truncate(3);
                lines[2] = format!("{}…", lines[2].trim_end_matches(['.', ',', ';', ':', ' ']));
            }
            let date = n.date.clone().unwrap_or_default();
            let w = lines
                .iter()
                .map(|l| text::width(l, SIZE) * if n.important { 1.06 } else { 1.0 })
                .chain([text::width(&date, DATE_SIZE)])
                .fold(40.0, f64::max)
                + 2.0 * PAD;
            let h = 2.0 * PAD + 14.0 + lines.len() as f64 * LINE;
            Some(Placed {
                node: n,
                span,
                bar: is_span(&date),
                lines,
                w,
                h,
            })
        })
        .collect();
    events.sort_by_key(|e| e.span.0);
    let mut p = Plot::default();
    let (t0, t1) = match (events.first(), events.iter().map(|e| e.span.1).max()) {
        (Some(f), Some(l)) => (f.span.0, l),
        _ => {
            p.w = g.w.unwrap_or(560.0);
            p.h = 80.0;
            p.marks.push(Mark::Line {
                x1: 0.0,
                y1: 40.0,
                x2: p.w,
                y2: 40.0,
                paint: Paint::Muted,
                width: 2.0,
            });
            p.marks.push(Mark::Text {
                x: p.w / 2.0,
                y: 28.0,
                text: "No events yet: add one per line, e.g. 2026-10-28: Open beta".into(),
                size: 12.0,
                anchor: "middle",
                paint: Paint::Muted,
                bold: false,
            });
            return p;
        }
    };
    // the scale: time, but every event gets at least MIN_GAP from the one
    // before, so busy weeks spread out and quiet years shrink. x_of runs
    // through each event's middle (knots) and is linear between them.
    let span = (t1 - t0).max(1) as f64;
    let widest = events.iter().map(|e| e.w).fold(0.0, f64::max);
    let target =
        g.w.unwrap_or_else(|| (events.len() as f64 * 110.0).clamp(560.0, 1600.0));
    let k = target / span; // px per day where events are sparse
    let margin = (widest / 2.0).min(90.0) + 10.0;
    let mut knots: Vec<(f64, f64)> = Vec::new(); // (time, x)
    for e in &events {
        let t = (e.span.0 + e.span.1) as f64 / 2.0;
        let x = match knots.last() {
            None => margin,
            Some(&(pt, px)) if t <= pt => px,
            Some(&(pt, px)) => px + ((t - pt) * k).clamp(MIN_GAP, MAX_GAP),
        };
        if knots.last().is_none_or(|&(pt, _)| t > pt) {
            knots.push((t, x));
        }
    }
    let x_of = |t: f64| -> f64 {
        let (first, last) = (knots[0], knots[knots.len() - 1]);
        if t <= first.0 {
            return first.1 - (first.0 - t) * k;
        }
        if t >= last.0 {
            return last.1 + (t - last.0) * k;
        }
        let i = knots.partition_point(|&(kt, _)| kt <= t);
        let ((ta, xa), (tb, xb)) = (knots[i - 1], knots[i]);
        xa + (t - ta) / (tb - ta) * (xb - xa)
    };
    let w = (knots[knots.len() - 1].1 + margin)
        .max(widest + 20.0)
        .max(target.min(560.0));
    // the line runs a little before the first and after the last event
    let (t0, t1) = (t0 as f64 - margin / k, t1 as f64 + margin / k);

    // lanes: alternate above and below; the nearest lane with room.
    // Important events go first, so they sit closest to the line.
    let mut lanes: [Vec<Vec<(f64, f64)>>; 2] = [Vec::new(), Vec::new()]; // boxes per lane
    let mut placed: Vec<(usize, usize, f64, f64)> = vec![(0, 0, 0.0, 0.0); events.len()];
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by_key(|&i| !events[i].node.important);
    let mut placed_so_far: Vec<(usize, usize, f64, f64, f64)> = Vec::new();
    let dots: Vec<f64> = events
        .iter()
        .map(|e| x_of((e.span.0 + e.span.1) as f64 / 2.0))
        .collect();
    let mut done = vec![false; events.len()];
    for i in order {
        let e = &events[i];
        let at = x_of((e.span.0 + e.span.1) as f64 / 2.0);
        let prefer = i % 2;
        // every place is scored: a label may not overlap another in its lane;
        // a connector crossing a label costs most, then covering the dot of
        // an event still to place (its connector may have to cross), then
        // distance from the line, the other side, a box off-centre
        let score = |side: usize, lane: usize, left: f64, shifted: bool| -> Option<f64> {
            let right = left + e.w;
            let mut cost = lane as f64 * 2.0
                + if side == prefer { 0.0 } else { 0.5 }
                + if shifted { 0.3 } else { 0.0 };
            for &(s2, l2, at2, x0, x1) in &placed_so_far {
                if s2 != side {
                    continue;
                }
                if l2 == lane && !(x1 + 8.0 <= left || right + 8.0 <= x0) {
                    return None;
                }
                if (l2 < lane && (x0 - 4.0..=x1 + 4.0).contains(&at))
                    || (l2 > lane && (left - 4.0..=right + 4.0).contains(&at2))
                {
                    cost += 20.0;
                }
            }
            for (j, &a2) in dots.iter().enumerate() {
                if j != i && !done[j] && (left - 4.0..=right + 4.0).contains(&a2) {
                    cost += 1.5;
                }
            }
            Some(cost)
        };
        // the box centred on its dot, or starting or ending at it
        let lefts = [
            (at - e.w / 2.0, false),
            (at - 2.0, true),
            (at - e.w + 2.0, true),
        ]
        .map(|(l, s)| (l.clamp(0.0, w - e.w), s));
        let mut best = (f64::INFINITY, prefer, 0, lefts[0].0);
        for side in [prefer, 1 - prefer] {
            for lane in 0..=lanes[side].len() {
                for &(left, shifted) in &lefts {
                    if let Some(c) = score(side, lane, left, shifted) {
                        if c < best.0 {
                            best = (c, side, lane, left);
                        }
                    }
                }
            }
        }
        let (_, side, lane, left) = best;
        done[i] = true;
        let right = left + e.w;
        if lanes[side].len() <= lane {
            lanes[side].resize(lane + 1, Vec::new());
        }
        lanes[side][lane].push((left, right));
        placed_so_far.push((side, lane, at, left, right));
        placed[i] = (side, lane, at, left);
    }
    let lane_h = events.iter().map(|e| e.h).fold(0.0, f64::max) + 10.0;
    let above = lanes[0].len() as f64;
    let below = lanes[1].len() as f64;
    let axis = above * lane_h + GAP;
    p.w = w;
    p.h = axis + GAP + 10.0 + below * lane_h;

    // the line and its ticks
    p.marks.push(Mark::Line {
        x1: 0.0,
        y1: axis,
        x2: w,
        y2: axis,
        paint: Paint::Muted,
        width: 2.0,
    });
    let mut last_tick = f64::NEG_INFINITY;
    for (t, label) in ticks(t0.ceil() as i64, t1.floor() as i64) {
        let x = x_of(t as f64);
        // ticks where the scale is squeezed would collide
        if x < 0.0 || x > w || x - last_tick < text::width(&label, 10.0) + 12.0 {
            continue;
        }
        last_tick = x;
        p.marks.push(Mark::Line {
            x1: x,
            y1: axis - 4.0,
            x2: x,
            y2: axis + 4.0,
            paint: Paint::Muted,
            width: 1.0,
        });
        p.marks.push(Mark::Text {
            x,
            y: axis + 17.0,
            text: label,
            size: 10.0,
            anchor: "middle",
            paint: Paint::Muted,
            bold: false,
        });
    }
    for (e, &(side, lane, at, left)) in events.iter().zip(&placed) {
        let n = e.node;
        let color = n
            .color
            .clone()
            .or_else(|| n.important.then(|| "red".to_string()));
        let paint = color.clone().map(Paint::Color).unwrap_or(Paint::Fg);
        let top = if side == 0 {
            axis - GAP - lane as f64 * lane_h - e.h
        } else {
            axis + GAP + 10.0 + lane as f64 * lane_h
        };
        let rect = Rect {
            x: left,
            y: top,
            w: e.w,
            h: e.h,
        };
        // from its place on the line to its label
        let (y1, y2) = if side == 0 {
            (rect.y + rect.h, axis)
        } else {
            (axis, rect.y)
        };
        p.marks.push(Mark::Line {
            x1: at,
            y1,
            x2: at,
            y2,
            paint: Paint::Grid,
            width: 1.0,
        });
        if color.is_some() {
            p.marks.push(Mark::Rect {
                r: rect,
                paint: Paint::Soft(color.clone().unwrap()),
                round: 6.0,
            });
        }
        // a span is a bar on the line
        if e.bar {
            let (x0, x1) = (x_of(e.span.0 as f64), x_of(e.span.1 as f64 + 1.0));
            p.marks.push(Mark::Rect {
                r: Rect {
                    x: x0,
                    y: axis - 4.0,
                    w: (x1 - x0).max(6.0),
                    h: 8.0,
                },
                paint: color.clone().map(Paint::Color).unwrap_or(Paint::Muted),
                round: 4.0,
            });
        }
        if n.important {
            p.marks.push(Mark::Path {
                d: star(at, axis, 10.0),
                stroke: Some(Paint::Bg),
                fill: Some(paint.clone()),
                width: 1.5,
                opacity: 1.0,
            });
        } else {
            p.marks.push(Mark::Circle {
                cx: at,
                cy: axis,
                r: 6.0,
                paint: Paint::Bg,
            });
            p.marks.push(Mark::Circle {
                cx: at,
                cy: axis,
                r: 4.5,
                paint: paint.clone(),
            });
        }
        p.marks.push(Mark::Text {
            x: rect.x + PAD,
            y: rect.y + PAD + DATE_SIZE,
            text: n.date.clone().unwrap_or_default(),
            size: DATE_SIZE,
            anchor: "start",
            paint: Paint::Muted,
            bold: false,
        });
        for (k, l) in e.lines.iter().enumerate() {
            p.marks.push(Mark::Text {
                x: rect.x + PAD,
                y: rect.y + PAD + 14.0 + k as f64 * LINE + SIZE,
                text: l.clone(),
                size: SIZE,
                anchor: "start",
                paint: Paint::Fg,
                bold: n.important,
            });
        }
        p.rows.push((n.id.clone(), rect));
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> i64 {
        parse_date(s).unwrap()
    }

    #[test]
    fn dates_as_people_write_them() {
        assert_eq!(
            parse("2026-10-28"),
            Some((d("2026-10-28"), d("2026-10-28")))
        );
        assert_eq!(
            parse("28 Oct 2026"),
            Some((d("2026-10-28"), d("2026-10-28")))
        );
        assert_eq!(
            parse("Oct 28, 2026"),
            Some((d("2026-10-28"), d("2026-10-28")))
        );
        assert_eq!(
            parse("18th June 2025"),
            Some((d("2025-06-18"), d("2025-06-18")))
        );
        assert_eq!(parse("Dec 2024"), Some((d("2024-12-01"), d("2024-12-31"))));
        assert_eq!(parse("2024-02"), Some((d("2024-02-01"), d("2024-02-29"))));
        assert_eq!(parse("Q3 2026"), Some((d("2026-07-01"), d("2026-09-30"))));
        assert_eq!(parse("2026"), Some((d("2026-01-01"), d("2026-12-31"))));
        assert_eq!(
            parse("7–14 May 2026"),
            Some((d("2026-05-07"), d("2026-05-14")))
        );
        assert_eq!(
            parse("7 May – 2 Jun 2026"),
            Some((d("2026-05-07"), d("2026-06-02")))
        );
        assert_eq!(
            parse("2026-05-07..2026-05-14"),
            Some((d("2026-05-07"), d("2026-05-14")))
        );
        assert_eq!(
            parse("Jan - Mar 2026"),
            Some((d("2026-01-01"), d("2026-03-31")))
        );
        for bad in ["", "soon", "32 Oct 2026", "Oct", "14–7 May 2026", "2026-13"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_language_round_trips_and_ops_check_dates() {
        let src = "title: T\n\ntimeline history: Field kit [blue] {\n  Dec 2024: Supplier: no markup on parts\n  18 Jun 2025: Store listing [detail: £5k per region]\n  7–14 May 2026: Regulator approves [id: visa]\n  2026-10-28: Open beta [important, red]\n  2027:\n}\n";
        let doc = crate::dsl::parse(src).unwrap();
        let g = doc.group("history").unwrap();
        assert!(g.timeline && g.color.as_deref() == Some("blue"));
        let ev = events_of(&doc, "history");
        assert_eq!(ev.len(), 5);
        assert_eq!(
            (
                ev[0].id.as_str(),
                ev[0].date.as_deref(),
                ev[0].label.as_str()
            ),
            (
                "history.supplier-no-markup-on-parts",
                Some("Dec 2024"),
                "Supplier: no markup on parts"
            )
        );
        assert_eq!(ev[2].id, "visa");
        assert!(ev[3].important && ev[3].color.as_deref() == Some("red"));
        assert_eq!(
            (ev[4].date.as_deref(), ev[4].label.as_str()),
            (Some("2027"), "")
        );
        let out = crate::dsl::print(&doc);
        assert!(out.contains("timeline history: Field kit [blue] {\n  Dec 2024: Supplier: no markup on parts\n"), "{out}");
        assert_eq!(crate::dsl::parse(&out).unwrap(), doc, "{out}");
        // as JSON, and through the patcher (rewriting only the block)
        assert_eq!(crate::from_json(&crate::to_json(&doc)).unwrap(), doc);
        let mut s = crate::Session::new();
        s.set_source(src);
        assert_eq!(
            s.apply(r#"{"op":"update_node","id":"visa","important":true,"date":"May 2026"}"#)["ok"],
            true
        );
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("  May 2026: Regulator approves [id: visa, important]\n")
                && out.starts_with("title: T\n"),
            "{out}"
        );
        // dates are checked
        assert!(
            crate::dsl::parse("timeline t: T {\n  soon: Launch\n}\n").unwrap_err()[0]
                .message
                .contains("cannot read the date `soon`")
        );
        assert!(crate::dsl::parse("timeline t: T {\n  Launch\n}\n").is_err());
        let e =
            s.apply(r#"{"op":"add_node","id":"x","label":"X","group":"history","date":"someday"}"#);
        assert!(
            e["error"]
                .as_str()
                .unwrap()
                .contains("cannot read the date"),
            "{e}"
        );
        assert_eq!(s.apply(r#"{"ops":[{"op":"add_group","id":"t2","label":"Plan","timeline":true},{"op":"add_node","id":"a","label":"Kick-off","group":"t2","date":"Q1 2027"}]}"#)["ok"], true);
        assert!(s
            .source("dsl")
            .unwrap()
            .contains("timeline t2: Plan {\n  Q1 2027: Kick-off [id: a]\n}"));
    }

    #[test]
    fn connectors_never_cross_a_label() {
        let doc = crate::dsl::parse("timeline t: T {\n  Dec 2024: Supplier: no markup on parts, only monthly rebates\n  18 Jun 2025: App store review priced at £5k per region (UK+EU one region)\n  20 Apr 2026: Mia's prototype teased at the company all-hands; a design mock with the new settings\n  7–14 May 2026: Regulator approves the design: NZ, AU, CA, UK\n  10 Jul 2026: Sam: the basic kit is easier; the pro kit needs more checks\n  17 Jul 2026: Ana takes the lead on the basic kit\n  28 Jul 2026: Pro kit pushed to Q4: the app revamp first\n  ~6 Aug 2026: First basic kit demo; free batch-of-20 pricing\n  17–19 Aug 2026: Pro kit spike (Lee); Kai's pricing discussion\n  31 Aug 2026: #kit-dev created\n  3 Sep 2026: The app store rejects the companion app (allowlist)\n  30 Sep 2026: Product rename thread (42 replies)\n  1–8 Oct 2026: Foundation PRs; live behind a feature flag\n  13 Oct 2026: Production check fails; safety reporting named a blocker\n  14 Oct 2026: First real order in production [green]\n  15 Oct 2026: Naming deck approved (naming superseded 17 Oct); one owner per kit\n  16 Oct 2026: Beta moved 19 → 28 Oct [red]\n  17 Oct 2026: Scope clarified: 28 Oct ships the basic kit; the pro kit is personalised, charged per kit, with remote setup [purple]\n}\n").unwrap();
        let p = plot(&doc, doc.group("t").unwrap());
        let lines: Vec<(f64, f64, f64)> = p
            .marks
            .iter()
            .filter_map(|m| match m {
                Mark::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    paint: Paint::Grid,
                    ..
                } if x1 == x2 => Some((*x1, y1.min(*y2), y1.max(*y2))),
                _ => None,
            })
            .collect();
        for (id, r) in &p.rows {
            for &(x, a, b) in &lines {
                let inside =
                    x > r.x + 1.0 && x < r.x + r.w - 1.0 && b > r.y + 1.0 && a < r.y + r.h - 1.0;
                assert!(!inside, "a connector at x={x:.0} crosses `{id}` {r:?}");
            }
        }
    }

    #[test]
    fn events_get_lanes_out_of_each_others_way() {
        let doc = crate::dsl::parse("timeline t: T {\n  2026-01-01: One\n  2026-01-02: Two\n  2026-01-03: Three\n  2026-01-04: Four [important]\n  2026-03-01..2026-04-01: Spring\n}\n").unwrap();
        let g = doc.group("t").unwrap();
        let p = plot(&doc, g);
        assert_eq!(p.rows.len(), 5);
        // no two labels overlap
        for (i, (_, a)) in p.rows.iter().enumerate() {
            for (_, b) in &p.rows[i + 1..] {
                let apart =
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                assert!(apart, "{a:?} {b:?}");
            }
        }
        assert!(
            p.marks.iter().any(|m| matches!(m, Mark::Path { .. })),
            "the important one is a star"
        );
        assert_eq!(
            p.marks
                .iter()
                .filter(|m| matches!(m, Mark::Rect { r, .. } if r.h == 8.0))
                .count(),
            1,
            "only the span is a bar"
        );
        assert!(is_span("7–14 May 2026") && !is_span("Dec 2024") && !is_span("mid Dec 2026"));
        let t: Vec<String> = ticks(d("2025-11-20"), d("2026-03-10"))
            .into_iter()
            .map(|t| t.1)
            .collect();
        assert_eq!(t, ["Dec 2025", "Jan 2026", "Feb", "Mar"]);
    }
}
