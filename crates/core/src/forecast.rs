//! Forecasts: a number per month (revenue, usually) as it is now and as
//! each launch adds to it, on the dates the plan gives the launches.
//!
//! ```text
//! forecast revenue: Revenue [unit: £] {
//!   now: 12k
//!   Phase 1 GA [adds: 3k, ramp: 3]
//!   Phase 2 GA [adds: 8k, ramp: 6]
//! }
//! ```
//!
//! A launch is a gantt milestone, by its name (or `on: DATE` when it is not
//! on a chart). From its date it adds a share of `adds` each month, all of
//! it after `ramp` months (at once without one). Drawn as two lines: as
//! planned (the milestones' planned dates) and as forecast (their forecast
//! dates, so a slip in the plan moves the curve); [`headline`] says where it
//! gets to and what the slips cost.
use crate::charts::Plot;
use crate::gantt;
use crate::model::{ChartKind, Document, Group, Node};

/// `12k` → 12000, `1.5m` → 1500000, `£3,000` → 3000.
pub fn parse_amount(s: &str) -> Option<f64> {
    let t: String = s
        .trim()
        .chars()
        .filter(|c| !matches!(c, '£' | '$' | '€' | '¥' | ',' | '_' | ' '))
        .collect();
    let (num, mul) = match t.chars().last()?.to_ascii_lowercase() {
        'k' => (&t[..t.len() - 1], 1e3),
        'm' => (&t[..t.len() - 1], 1e6),
        'b' => (&t[..t.len() - 1], 1e9),
        _ => (t.as_str(), 1.0),
    };
    num.parse::<f64>().ok().map(|v| v * mul)
}

/// 12000 → `12k`, 1500 → `1.5k`, 1250 → `1250`: short, and read back the same.
pub fn format_amount(v: f64) -> String {
    let clean = |x: f64| (x * 10.0).round() == x * 10.0;
    for (div, suffix) in [(1e9, "b"), (1e6, "m"), (1e3, "k")] {
        if v.abs() >= div && clean(v / div) {
            return format!("{}{suffix}", crate::text::n(v / div));
        }
    }
    crate::text::n(v)
}

/// A number with the forecast's unit: `£12k`, `340 boards`.
pub fn with_unit(v: f64, unit: Option<&str>) -> String {
    let n = format_amount(nice(v));
    match unit {
        Some(u) if matches!(u, "£" | "$" | "€" | "¥") => format!("{u}{n}"),
        Some(u) if !u.is_empty() => format!("{n} {u}"),
        _ => n,
    }
}

/// `v` rounded to what reads well: hundreds from 10k, tens from 1k, whole below.
fn nice(v: f64) -> f64 {
    let step = if v.abs() >= 10_000.0 {
        100.0
    } else if v.abs() >= 1000.0 {
        10.0
    } else {
        1.0
    };
    (v / step).round() * step
}

/// The launches of forecast `g`, in order.
pub fn launches<'a>(doc: &'a Document, g: &str) -> Vec<&'a Node> {
    doc.nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g))
        .collect()
}

/// The month-by-month numbers of a forecast.
#[derive(Debug, Clone)]
pub struct Series {
    /// The first day of each month.
    pub months: Vec<i64>,
    pub planned: Vec<f64>,
    pub forecast: Vec<f64>,
    /// Each launch: its name, planned and forecast day (None when it has no date).
    pub launches: Vec<(String, Option<(i64, i64)>)>,
}

/// The first of the month `z` is in.
fn month_start(z: i64) -> i64 {
    let (y, m, _) = gantt::civil(z);
    gantt::parse_date(&format!("{y:04}-{m:02}-01")).unwrap_or(z)
}

/// The first of the month after `z`'s.
fn next_month(z: i64) -> i64 {
    month_start(month_start(z) + 32)
}

/// Forecast `g` (its launches in `doc`), with launch dates looked up in
/// `plan` (the whole board), month by month from `today`'s.
pub fn series(plan: &Document, doc: &Document, g: &Group, today: i64) -> Series {
    let ms = crate::dashboard::milestones(plan, today);
    let launches: Vec<(String, Option<(i64, i64)>, f64, f64)> = launches(doc, &g.id)
        .into_iter()
        .map(|n| {
            let name = n.display().to_string();
            let found = ms
                .iter()
                .find(|m| m.label.eq_ignore_ascii_case(&name) || m.at.id == name)
                .map(|m| (m.due, m.forecast))
                .or_else(|| {
                    n.date
                        .as_deref()
                        .and_then(gantt::parse_date)
                        .map(|z| (z, z))
                });
            (name, found, n.adds.unwrap_or(0.0), n.ramp.unwrap_or(0.0))
        })
        .collect();
    let base = g.now.unwrap_or(0.0);
    // from this month until everything has ramped up, and a little after
    let first = month_start(today);
    let end = launches
        .iter()
        .filter_map(|(_, d, _, r)| d.map(|(p, f)| p.max(f) + (r * 30.44) as i64))
        .max()
        .unwrap_or(first);
    let mut months = vec![first];
    while (*months.last().unwrap() < end || months.len() < 7) && months.len() < 25 {
        months.push(next_month(*months.last().unwrap()));
    }
    months.push(next_month(*months.last().unwrap()));
    let value = |at: i64, pick: fn((i64, i64)) -> i64| -> f64 {
        base + launches
            .iter()
            .filter_map(|(_, d, adds, ramp)| d.map(|d| (pick(d), *adds, *ramp)))
            .map(|(day, adds, ramp)| {
                let months_in = (at - day) as f64 / 30.44;
                if months_in < 0.0 {
                    0.0
                } else if ramp <= 0.0 {
                    adds
                } else {
                    adds * (months_in / ramp).min(1.0)
                }
            })
            .sum::<f64>()
    };
    let planned = months.iter().map(|&m| value(m, |d| d.0)).collect();
    let forecast = months.iter().map(|&m| value(m, |d| d.1)).collect();
    Series {
        months,
        planned,
        forecast,
        launches: launches.into_iter().map(|(n, d, _, _)| (n, d)).collect(),
    }
}

fn month_name(z: i64, with_year: bool) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (y, m, _) = gantt::civil(z);
    if with_year {
        format!("{} {}", MONTHS[(m - 1) as usize], y)
    } else {
        MONTHS[(m - 1) as usize].to_string()
    }
}

/// "£12k a month now → £23k by Jun 2027 · the slips cost £4k by then", or
/// "… · on plan".
pub fn headline(s: &Series, unit: Option<&str>, now: f64) -> String {
    let (Some(&last), Some(&end)) = (s.forecast.last(), s.months.last()) else {
        return String::new();
    };
    let lost: f64 = s
        .planned
        .iter()
        .zip(&s.forecast)
        .map(|(p, f)| (p - f).max(0.0))
        .sum();
    let money = matches!(unit, Some("£" | "$" | "€" | "¥"));
    let a_month = if money { " a month" } else { "" };
    let mut out = format!(
        "{}{a_month} now → {} by {}",
        with_unit(now, unit),
        with_unit(last, unit),
        month_name(end, true)
    );
    // money lost adds up month by month; anything else is behind by the widest gap
    let gap = s
        .planned
        .iter()
        .zip(&s.forecast)
        .map(|(p, f)| p - f)
        .fold(0.0_f64, f64::max);
    if money && nice(lost) >= 1.0 {
        out.push_str(&format!(
            " · the slips cost {} by then",
            with_unit(lost, unit)
        ));
    } else if !money && nice(gap) >= 1.0 {
        out.push_str(&format!(
            " · the slips put it up to {} behind plan",
            with_unit(gap, unit)
        ));
    } else {
        out.push_str(" · on plan");
    }
    let missing: Vec<&str> = s
        .launches
        .iter()
        .filter(|(_, d)| d.is_none())
        .map(|(n, _)| n.as_str())
        .collect();
    if !missing.is_empty() {
        out.push_str(&format!(" · no date for {}", missing.join(", ")));
    }
    out
}

/// Forecast `g` as a line chart: planned and forecast, month by month.
pub fn as_chart(plan: &Document, doc: &Document, g: &Group, today: i64) -> (Group, Vec<Node>) {
    let s = series(plan, doc, g, today);
    let mut chart = Group {
        id: g.id.clone(),
        label: g.label.clone(),
        chart: Some(ChartKind::Line),
        series: vec!["Planned".into(), "Forecast".into()],
        unit: g.unit.clone(),
        color: g.color.clone(),
        w: Some(g.w.unwrap_or(560.0)),
        ..Default::default()
    };
    if chart.label.is_empty() {
        chart.label = g.id.clone();
    }
    let rows = s
        .months
        .iter()
        .enumerate()
        .map(|(i, &m)| {
            let mut n = Node::new(&format!("{}.m{i}", g.id));
            let (_, mo, _) = gantt::civil(m);
            n.label = month_name(m, i == 0 || mo == 1);
            n.group = Some(g.id.clone());
            n.values = vec![Some(s.planned[i].round()), Some(s.forecast[i].round())];
            n
        })
        .collect();
    (chart, rows)
}

/// Forecast `g` drawn on the board: as a chart of its months.
pub fn plot(doc: &Document, g: &Group) -> Plot {
    let today = crate::todo::today()
        .unwrap_or_else(|| month_start(gantt::parse_date("2026-01-01").unwrap_or(0)));
    let (chart, rows) = as_chart(doc, doc, g, today);
    let tmp = Document {
        nodes: rows,
        groups: vec![chart.clone()],
        ..Default::default()
    };
    crate::charts::plot(&tmp, &chart)
}

/// Forecast `gid`'s lines as text: `now: 12k`, then a launch per line.
pub fn text_of(doc: &Document, gid: &str) -> Option<String> {
    let g = doc.group(gid).filter(|g| g.forecast)?;
    let mut out = String::new();
    crate::dsl::forecast_block(doc, g, "", &mut out);
    let body: Vec<&str> = out.lines().skip(1).collect();
    Some(
        body[..body.len().saturating_sub(1)]
            .iter()
            .map(|l| l.trim_start())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// The ops that make forecast `gid` what `text` says (its lines, as
/// [`text_of`] gives them).
pub fn edit_ops(doc: &Document, gid: &str, text: &str) -> Result<Vec<serde_json::Value>, String> {
    use serde_json::json;
    doc.group(gid)
        .filter(|g| g.forecast)
        .ok_or_else(|| format!("no forecast `{gid}`"))?;
    let body: String = text.lines().map(|l| format!("  {}\n", l.trim())).collect();
    let fresh = crate::dsl::parse(&format!("forecast {gid} {{\n{body}}}\n")).map_err(|e| {
        e.iter()
            .map(|d| format!("line {}: {}", d.line.saturating_sub(1), d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let g = fresh.group(gid).unwrap();
    let mut ops = vec![json!({"op": "update_group", "id": gid, "now": g.now.unwrap_or(0.0)})];
    for n in launches(doc, gid) {
        ops.push(json!({"op": "remove", "id": n.id}));
    }
    for n in launches(&fresh, gid) {
        let mut op = json!({"op": "add_node", "id": n.id, "group": gid, "label": n.display()});
        let o = op.as_object_mut().unwrap();
        if let Some(v) = n.adds {
            o.insert("adds".into(), json!(v));
        }
        if let Some(v) = n.ramp {
            o.insert("ramp".into(), json!(v));
        }
        if let Some(v) = &n.date {
            o.insert("date".into(), json!(v));
        }
        if let Some(v) = &n.color {
            o.insert("color".into(), json!(v));
        }
        if let Some(v) = &n.detail {
            o.insert("detail".into(), json!(v));
        }
        ops.push(op);
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_read_and_print_back() {
        assert_eq!(parse_amount("12k"), Some(12000.0));
        assert_eq!(parse_amount("£1.5m"), Some(1_500_000.0));
        assert_eq!(parse_amount("3,000"), Some(3000.0));
        assert_eq!(parse_amount("lots"), None);
        for v in [12000.0, 1500.0, 1250.0, 250.0, 2_000_000.0, 0.0] {
            assert_eq!(parse_amount(&format_amount(v)), Some(v), "{v}");
        }
    }

    const BOARD: &str = "title: Launch\n\n\
        gantt road: Roadmap [start: 2026-10-01] {\n\
        \x20 Build [2026-10-01, 4w, done: 10%]\n\
        \x20 GA [2026-10-30, milestone]\n\
        }\n\
        forecast revenue: Revenue [unit: £] {\n\
        \x20 now: 10k\n\
        \x20 GA [adds: 6k, ramp: 3]\n\
        \x20 Partners [adds: 2k, on: 2027-01-01]\n\
        }\n";

    #[test]
    fn a_slip_in_the_plan_moves_the_curve() {
        let doc = crate::dsl::parse(BOARD).unwrap();
        assert_eq!(crate::dsl::print(&doc).contains("forecast revenue: Revenue [unit: £] {\n  now: 10k\n  GA [adds: 6k, ramp: 3]\n  Partners [adds: 2k, on: 2027-01-01]\n}"), true, "{}", crate::dsl::print(&doc));
        assert_eq!(crate::dsl::parse(&crate::dsl::print(&doc)).unwrap(), doc);
        let today = gantt::parse_date("2026-10-15").unwrap();
        let g = doc.group("revenue").unwrap();
        let s = series(&doc, &doc, g, today);
        // October: nothing launched yet; three months after GA all of it, plus partners
        assert_eq!(s.planned[0], 10_000.0);
        assert_eq!(*s.planned.last().unwrap(), 18_000.0);
        // Build is 10% done with two weeks left: GA is forecast late, the curve lags
        assert!(s.forecast.iter().zip(&s.planned).any(|(f, p)| f < p));
        let h = headline(&s, Some("£"), 10_000.0);
        assert!(h.starts_with("£10k a month now → £18k by "), "{h}");
        assert!(h.contains("the slips cost £"), "{h}");
        // drawn as a chart
        let svg = crate::render::render(&doc, &Default::default()).svg;
        assert!(svg.contains("Forecast"));
        // edited as text
        assert_eq!(
            text_of(&doc, "revenue").unwrap(),
            "now: 10k\nGA [adds: 6k, ramp: 3]\nPartners [adds: 2k, on: 2027-01-01]"
        );
        let ops = edit_ops(&doc, "revenue", "now: 12k\nGA [adds: 7k, ramp: 2]").unwrap();
        let mut d = doc.clone();
        crate::ops::apply(
            &mut d,
            &crate::ops::parse_ops(&serde_json::json!({ "ops": ops }).to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            text_of(&d, "revenue").unwrap(),
            "now: 12k\nGA [adds: 7k, ramp: 2]"
        );
    }
}
