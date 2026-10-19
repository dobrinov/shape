//! Dashboards: the first screen of a perspective, computed from the whole
//! board (its canvases too), so it never needs keeping up to date.
//!
//! Each perspective answers its audience's first question at a glance
//! (Delivery: are we on plan? Business: when does it sell, and what do we
//! need to decide?) with a headline and a few tables: milestones with a
//! traffic light and the reason for it, blockers, decisions, top risks,
//! what changed since an earlier version. Every row stands for an element
//! of the board: its id is [`row_id`], and the editor opens that element
//! when the row is clicked.
//!
//! A milestone's light:
//! - **red**: the work planned to end by it is forecast to end after it,
//!   or is late; the milestone itself is forecast late; a blocker or a risk
//!   scoring 20 of 25 or more is linked to it by an arrow;
//! - **amber**: work before it is behind; it is the next milestone and the
//!   board has open blockers, a risk scoring 12 or more or overdue to-dos
//!   that no arrow ties to another milestone;
//! - **green** otherwise.
use crate::gantt;
use crate::model::{Document, Group, LayoutKind, Node, Perspective};

/// A traffic light.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Light {
    Green,
    Amber,
    Red,
}

impl Light {
    pub fn word(self) -> &'static str {
        match self {
            Light::Green => "On track",
            Light::Amber => "At risk",
            Light::Red => "Off track",
        }
    }
    fn color(self) -> &'static str {
        match self {
            Light::Green => "green",
            Light::Amber => "orange",
            Light::Red => "red",
        }
    }
}

/// Where an element is: the canvas path (empty for the top board) and its id.
#[derive(Debug, Clone, PartialEq)]
pub struct At {
    pub path: Vec<String>,
    pub id: String,
}

/// An upcoming milestone and how it stands.
#[derive(Debug, Clone)]
pub struct Milestone {
    pub at: At,
    pub label: String,
    /// Planned day (a day number, see [`gantt::parse_date`]).
    pub due: i64,
    pub light: Light,
    /// Why, in a line ("" when on track).
    pub reason: String,
    /// The gantt chart it is on, and the work planned to end by it (ids).
    pub chart: String,
    pub work: Vec<String>,
    /// When it is forecast to happen: its own forecast, or later when the
    /// work planned to end by it is forecast to end after it.
    pub forecast: i64,
}

/// A blocker, a decision or a risk.
#[derive(Debug, Clone)]
pub struct Item {
    pub at: At,
    pub label: String,
    pub owner: String,
    /// A due date, as written.
    pub due: String,
    /// Risks: probability × impact, out of 25.
    pub score: f64,
    /// Where it is on the board ("Blockers & risks").
    pub place: String,
}

/// The id of a dashboard row that stands for element `at` (`tile` keeps
/// rows of different tables apart): `@dash/TILE/PATH:ID`.
pub fn row_id(tile: &str, at: &At) -> String {
    format!("@dash/{tile}/{}:{}", at.path.join("/"), at.id)
}

/// The element a dashboard row stands for: (canvas path, id).
pub fn target(row: &str) -> Option<(Vec<String>, String)> {
    let rest = row.strip_prefix("@dash/")?;
    let (_, rest) = rest.split_once('/')?;
    let (path, id) = rest.rsplit_once(':')?;
    let path = path
        .split('/')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect();
    Some((path, id.to_string()))
}

/// Every document of the board with its canvas path, the top board first.
fn docs(doc: &Document) -> Vec<(Vec<String>, &Document)> {
    fn walk<'a>(d: &'a Document, path: Vec<String>, out: &mut Vec<(Vec<String>, &'a Document)>) {
        out.push((path.clone(), d));
        for c in &d.canvases {
            let mut p = path.clone();
            p.push(c.id.clone());
            walk(&c.doc, p, out);
        }
    }
    let mut out = Vec::new();
    walk(doc, Vec::new(), &mut out);
    out
}

fn place(doc: &Document, path: &[String], d: &Document, n: &Node) -> String {
    let block = n
        .group
        .as_deref()
        .and_then(|g| d.group(g))
        .map(|g| g.display().to_string());
    let canvas = (!path.is_empty())
        .then(|| {
            doc.at(&path[..path.len() - 1])
                .and_then(|p| p.canvas(path.last().unwrap()))
        })
        .flatten()
        .map(|c| c.display().to_string());
    match (canvas, block) {
        (Some(c), Some(b)) if c != b => format!("{c} › {b}"),
        (Some(c), _) => c,
        (None, Some(b)) => b,
        (None, None) => String::new(),
    }
}

/// The groups `n` is in, innermost first.
fn ancestors<'a>(d: &'a Document, n: &Node) -> Vec<&'a Group> {
    let mut out = Vec::new();
    let mut g = n.group.as_deref().and_then(|g| d.group(g));
    while let Some(x) = g {
        out.push(x);
        g = x.parent.as_deref().and_then(|p| d.group(p));
    }
    out
}

fn word_in(text: &str, words: &[&str]) -> bool {
    let t = text.to_lowercase();
    words.iter().any(|w| t.contains(w))
}

fn done(n: &Node) -> bool {
    n.done.is_some_and(|d| d >= 100.0)
}

/// Open blockers: cards of a blockers kanban (not in a done column),
/// what sits in a section about blockers or what is at risk, and anything
/// tagged `blocker`.
pub fn blockers(doc: &Document) -> Vec<Item> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for n in &d.nodes {
            if done(n) || n.column.is_some() {
                continue;
            }
            let up = ancestors(d, n);
            if up.iter().any(|g| g.risks || g.gantt || g.plotted()) {
                continue;
            }
            let closed = up.iter().any(|g| {
                word_in(
                    g.display(),
                    &["done", "solved", "closed", "resolved", "fixed"],
                )
            });
            let about = n.tags.iter().any(|t| t == "blocker" || t == "blocked")
                || up
                    .iter()
                    .any(|g| word_in(g.display(), &["blocker", "blocked", "at risk"]));
            if about && !closed {
                out.push(item(doc, &path, d, n));
            }
        }
    }
    out
}

/// Open decisions: proposed decision records, what sits in a section of
/// asks or decisions, and anything tagged `decision`.
pub fn decisions(doc: &Document) -> Vec<Item> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for g in &d.groups {
            if g.adr.as_deref() == Some("proposed") {
                let mut it = Item {
                    at: At {
                        path: path.clone(),
                        id: g.id.clone(),
                    },
                    label: g.display().to_string(),
                    owner: String::new(),
                    due: String::new(),
                    score: 0.0,
                    place: String::new(),
                };
                it.place = path
                    .last()
                    .and_then(|c| doc.at(&path[..path.len() - 1]).and_then(|p| p.canvas(c)))
                    .map(|c| c.display().to_string())
                    .unwrap_or_default();
                out.push(it);
            }
        }
        for n in &d.nodes {
            if done(n) {
                continue;
            }
            let up = ancestors(d, n);
            if up.iter().any(|g| g.adr.is_some() || g.gantt || g.risks) {
                continue;
            }
            let about = n.tags.iter().any(|t| t == "decision" || t == "ask")
                || up.iter().any(|g| {
                    word_in(
                        g.display(),
                        &[
                            "need from",
                            "decision",
                            "to decide",
                            "asks",
                            "approval",
                            "sign-off",
                            "sign off",
                        ],
                    )
                });
            if about {
                out.push(item(doc, &path, d, n));
            }
        }
    }
    out.sort_by(|a, b| (a.due.is_empty(), a.due.clone()).cmp(&(b.due.is_empty(), b.due.clone())));
    out
}

/// Risks from the risk matrices, worst first.
pub fn risks(doc: &Document) -> Vec<Item> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for n in &d.nodes {
            let Some(g) = n
                .group
                .as_deref()
                .and_then(|g| d.group(g))
                .filter(|g| g.risks)
            else {
                continue;
            };
            let (Some(p), Some(i)) = (n.probability, n.impact) else {
                continue;
            };
            let levels = f64::from(g.levels.unwrap_or(5));
            let mut it = item(doc, &path, d, n);
            it.score = f64::from(p) * f64::from(i) * 25.0 / (levels * levels);
            out.push(it);
        }
    }
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out
}

/// To-do items past their due date and not done.
pub fn overdue(doc: &Document, today: i64) -> Vec<Item> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for n in &d.nodes {
            let in_list = n
                .group
                .as_deref()
                .and_then(|g| d.group(g))
                .is_some_and(|g| g.todo);
            let due = n.date.as_deref().and_then(gantt::parse_date);
            if in_list && !done(n) && due.is_some_and(|z| z < today) {
                out.push(item(doc, &path, d, n));
            }
        }
    }
    out
}

fn item(doc: &Document, path: &[String], d: &Document, n: &Node) -> Item {
    Item {
        at: At {
            path: path.to_vec(),
            id: n.id.clone(),
        },
        label: n.display().to_string(),
        owner: n.owner.clone().unwrap_or_default(),
        due: n.date.clone().unwrap_or_default(),
        score: 0.0,
        place: place(doc, path, d, n),
    }
}

/// Is `a` tied to `b` by an arrow (either way, also through the groups they are in)?
fn linked(d: &Document, a: &str, b: &str) -> bool {
    let around = |id: &str| -> Vec<String> {
        let mut v = vec![id.to_string()];
        if let Some(n) = d.node(id) {
            v.extend(ancestors(d, n).into_iter().map(|g| g.id.clone()));
        }
        v
    };
    let (xa, xb) = (around(a), around(b));
    d.edges.iter().any(|e| {
        (xa.contains(&e.from) && xb.contains(&e.to)) || (xb.contains(&e.from) && xa.contains(&e.to))
    })
}

/// The upcoming milestones of every gantt chart (or a chart's end, when it
/// has none), soonest first, with their lights.
pub fn milestones(doc: &Document, today: i64) -> Vec<Milestone> {
    let mut out = Vec::new();
    for (path, d) in docs(doc) {
        for g in d.groups.iter().filter(|g| g.gantt) {
            let tracks = gantt::track(d, &g.id, today);
            let day = |s: &str| gantt::parse_date(s).unwrap_or(i64::MIN);
            let mut marks: Vec<&gantt::Track> = tracks.iter().filter(|t| t.milestone).collect();
            let end_row;
            if marks.is_empty() {
                // a chart without milestones: its end
                let Some(last) = tracks.iter().max_by_key(|t| day(&t.end)) else {
                    continue;
                };
                end_row = gantt::Track {
                    id: g.id.clone(),
                    ..last.clone()
                };
                marks.push(&end_row);
            }
            marks.sort_by_key(|m| day(&m.end));
            let mut prev = today - 60;
            for m in marks {
                let due = day(&m.end);
                let from = std::mem::replace(&mut prev, due);
                if due < today || m.status == "done" {
                    continue;
                }
                let label = d
                    .node(&m.id)
                    .map(|n| n.display().to_string())
                    .unwrap_or_else(|| format!("{} done", g.display()));
                let mut light = Light::Green;
                let mut why: Vec<(Light, String)> = Vec::new();
                if m.slip > 0 {
                    why.push((
                        Light::Red,
                        format!(
                            "forecast {}, {} days late",
                            gantt::short_date(day(&m.forecast_end)),
                            m.slip
                        ),
                    ));
                }
                let work: Vec<String> = tracks
                    .iter()
                    .filter(|t| !t.milestone && day(&t.end) <= due && day(&t.end) > from)
                    .map(|t| t.id.clone())
                    .collect();
                let forecast = tracks
                    .iter()
                    .filter(|t| work.contains(&t.id) && t.status != "done")
                    .map(|t| day(&t.forecast_end))
                    .chain([day(&m.forecast_end), due])
                    .max()
                    .unwrap_or(due);
                // the work planned to be done by then (since the milestone before)
                for t in tracks.iter().filter(|t| {
                    !t.milestone && t.status != "done" && day(&t.end) <= due && day(&t.end) > from
                }) {
                    let name = d
                        .node(&t.id)
                        .map(|n| n.display().to_string())
                        .unwrap_or_default();
                    let pct = t
                        .done
                        .map(|p| format!(", {p:.0}% done"))
                        .unwrap_or_default();
                    if t.status == "late" {
                        why.push((Light::Red, format!("{name} is late{pct}")));
                    } else if day(&t.forecast_end) > due {
                        why.push((
                            Light::Red,
                            format!(
                                "{name} forecast to end {}{pct}",
                                gantt::short_date(day(&t.forecast_end))
                            ),
                        ));
                    } else if t.status == "behind" {
                        why.push((Light::Amber, format!("{name} is behind{pct}")));
                    }
                }
                for b in blockers(doc)
                    .iter()
                    .filter(|b| b.at.path == path && linked(d, &b.at.id, &m.id))
                {
                    why.push((Light::Red, format!("blocked: {}", b.label)));
                }
                for r in risks(doc)
                    .iter()
                    .filter(|r| r.at.path == path && linked(d, &r.at.id, &m.id) && r.score >= 12.0)
                {
                    let l = if r.score >= 20.0 {
                        Light::Red
                    } else {
                        Light::Amber
                    };
                    why.push((l, format!("risk: {}", r.label)));
                }
                why.sort_by(|a, b| b.0.cmp(&a.0));
                if let Some((l, _)) = why.first() {
                    light = *l;
                }
                out.push(Milestone {
                    at: At {
                        path: path.clone(),
                        id: m.id.clone(),
                    },
                    label,
                    due,
                    light,
                    chart: g.id.clone(),
                    forecast,
                    work,
                    reason: why
                        .into_iter()
                        .map(|(_, s)| s)
                        .take(2)
                        .collect::<Vec<_>>()
                        .join("; "),
                });
            }
        }
    }
    out.sort_by_key(|m| m.due);
    // what no arrow ties to a milestone weighs on the next one
    if let Some(next) = out.first_mut() {
        let mut why = Vec::new();
        let tied = |at: &At| {
            doc.at(&at.path).is_some_and(|d| {
                d.groups.iter().any(|g| g.gantt)
                    && d.edges.iter().any(|e| e.from == at.id || e.to == at.id)
            })
        };
        let open: Vec<Item> = blockers(doc).into_iter().filter(|b| !tied(&b.at)).collect();
        if !open.is_empty() {
            why.push(format!(
                "{} open {}",
                open.len(),
                if open.len() == 1 {
                    "blocker"
                } else {
                    "blockers"
                }
            ));
        }
        let high = risks(doc)
            .into_iter()
            .filter(|r| r.score >= 12.0 && !tied(&r.at))
            .count();
        if high > 0 {
            why.push(format!(
                "{high} high {}",
                if high == 1 { "risk" } else { "risks" }
            ));
        }
        let late = overdue(doc, today).len();
        if late > 0 {
            why.push(format!(
                "{late} overdue {}",
                if late == 1 { "to-do" } else { "to-dos" }
            ));
        }
        if !why.is_empty() {
            if next.light == Light::Green {
                next.light = Light::Amber;
            }
            let more = why.join(", ");
            next.reason = if next.reason.is_empty() {
                more
            } else {
                format!("{}; {more}", next.reason)
            };
        }
    }
    out
}

/// "in 9 days", "tomorrow", "today".
fn countdown(days: i64) -> String {
    match days {
        0 => "today".into(),
        1 => "tomorrow".into(),
        n => format!("in {n} days"),
    }
}

/// What changed between `old` and `new`, in a few lines: (what, where).
pub fn since(old: &Document, new: &Document) -> Vec<(At, String)> {
    let mut out = Vec::new();
    for (path, d) in docs(new) {
        let Some(o) = old.at(&path) else {
            continue;
        };
        for e in crate::diff::diff(o, d).entries {
            if e.kind != "node" {
                continue;
            }
            let label = d
                .node(&e.id)
                .or_else(|| o.node(&e.id))
                .map(|n| n.display().to_string())
                .unwrap_or_else(|| e.id.clone());
            let what = match e.change {
                crate::render::Change::Added => format!("Added: {label}"),
                crate::render::Change::Removed => format!("Removed: {label}"),
                _ => {
                    let f: Vec<&str> = e.fields.iter().map(|(k, _, _)| k.as_str()).collect();
                    if f.iter().any(|k| matches!(*k, "start" | "days" | "after")) {
                        format!("Moved: {label}")
                    } else if f.iter().any(|k| matches!(*k, "probability" | "impact")) {
                        format!("Re-rated: {label}")
                    } else if f.contains(&"done") {
                        format!("Progress: {label}")
                    } else {
                        format!("Changed: {label}")
                    }
                }
            };
            out.push((
                At {
                    path: path.clone(),
                    id: e.id,
                },
                what,
            ));
        }
    }
    out
}

/// The tiles a perspective shows, in order.
fn tiles(id: &str) -> &'static [&'static str] {
    match id {
        "delivery" => &["burndown", "kanban", "gantt", "matrix"],
        "business" => &["money", "milestones", "decisions", "risks"],
        "marketing" => &["milestones", "todos", "decisions", "risks"],
        "product" => &["scope", "plan", "decisions", "screens"],
        "design" => &["screens", "decisions", "comments"],
        "tech" => &["records", "unrecorded", "blockers", "risks"],
        "legal" => &["records", "decisions", "risks"],
        "users" => &["screens", "comments", "decisions"],
        _ => &["milestones", "decisions", "risks", "blockers"],
    }
}

fn row(id: String, group: &str, label: &str, cells: &[String], color: Option<&str>) -> Node {
    let mut n = Node::new(&id);
    n.label = label.to_string();
    n.group = Some(group.to_string());
    n.cells = cells.to_vec();
    n.color = color.map(String::from);
    n
}

fn table(id: &str, label: &str, columns: &[&str]) -> Group {
    Group {
        id: id.to_string(),
        label: label.to_string(),
        table: true,
        columns: columns.iter().map(|c| c.to_string()).collect(),
        ..Default::default()
    }
}

/// The dashboard of perspective `p` for the whole board `doc`, as of day
/// `today`; `before` is an earlier version, for what changed since.
pub fn build(doc: &Document, p: &Perspective, today: i64, before: Option<&Document>) -> Document {
    let mut out = Document {
        title: Some(p.label.clone()),
        ..Default::default()
    };
    out.settings.layout = LayoutKind::Grid;
    let mut money_found = false;
    let ms = milestones(doc, today);
    let worst = ms.iter().take(3).map(|m| m.light).max();
    let next = ms.first();
    // the headline: the answer first
    let (head, color, detail) = match (p.id.as_str(), next) {
        ("delivery", Some(m)) => (
            format!("Now delivering: {}", m.label),
            m.light.color(),
            format!(
                "{} · due {}, {}{}",
                m.light.word(),
                gantt::short_date(m.due),
                countdown(m.due - today),
                if m.reason.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", m.reason)
                }
            ),
        ),
        ("business", _) if first_forecast(doc).is_some() => {
            let (d, g) = first_forecast(doc).unwrap();
            let s = crate::forecast::series(doc, d, g, today);
            let slipping = s.forecast.iter().zip(&s.planned).any(|(f, p)| f + 0.5 < *p);
            (
                format!(
                    "{}: {}",
                    g.display(),
                    crate::forecast::headline(&s, g.unit.as_deref(), g.now.unwrap_or(0.0))
                ),
                if slipping { "orange" } else { "green" },
                next.map(|m| {
                    format!(
                        "Next launch: {} {}, {}{}",
                        m.label,
                        countdown(m.due - today),
                        m.light.word().to_lowercase(),
                        if m.reason.is_empty() {
                            String::new()
                        } else {
                            format!(" · {}", m.reason)
                        }
                    )
                })
                .unwrap_or_default(),
            )
        }
        ("tech", _) => {
            let records = docs(doc)
                .iter()
                .map(|(_, d)| d.groups.iter().filter(|g| g.adr.is_some()).count())
                .sum::<usize>();
            let missing = decisions(doc)
                .into_iter()
                .filter(|x| {
                    doc.at(&x.at.path)
                        .and_then(|d| d.group(&x.at.id))
                        .is_none_or(|g| g.adr.is_none())
                })
                .count();
            (
                format!(
                    "Architecture decisions: {records} recorded{}",
                    if missing > 0 {
                        format!(", {missing} not written down yet")
                    } else {
                        String::new()
                    }
                ),
                if missing > 0 { "orange" } else { "green" },
                if missing > 0 {
                    "Each decision taken belongs in a decision record (Blocks → Decision record): context, options, decision, consequences".to_string()
                } else {
                    String::new()
                },
            )
        }
        ("product", _) => (
            format!(
                "What we are building: {}",
                doc.title.clone().unwrap_or_else(|| p.label.clone())
            ),
            "blue",
            doc.description.clone().unwrap_or_default(),
        ),
        ("business" | "marketing", Some(m)) => (
            format!(
                "Next launch: {} {}, {}",
                m.label,
                countdown(m.due - today),
                m.light.word().to_lowercase()
            ),
            m.light.color(),
            m.reason.clone(),
        ),
        (_, Some(m))
            if matches!(p.id.as_str(), "delivery" | "product" | "tech")
                || tiles(&p.id).contains(&"milestones") =>
        {
            (
                format!(
                    "{}: {} {}",
                    worst.unwrap_or(Light::Green).word(),
                    m.label,
                    countdown(m.due - today)
                ),
                worst.unwrap_or(Light::Green).color(),
                m.reason.clone(),
            )
        }
        _ => (
            doc.title.clone().unwrap_or_else(|| p.label.clone()),
            "gray",
            crate::perspectives::purpose(&p.id)
                .unwrap_or_default()
                .to_string(),
        ),
    };
    let mut h = Node::new("@dash/head/:");
    h.label = head;
    h.color = Some(color.into());
    if !detail.is_empty() {
        h.detail = Some(detail);
    }
    h.w = Some(720.0);
    out.nodes.push(h);

    for tile in tiles(&p.id) {
        match *tile {
            "milestones" if !ms.is_empty() => {
                out.groups.push(table(
                    "milestones",
                    "Milestones",
                    &["Milestone", "When", "Status", "Why"],
                ));
                for m in ms.iter().take(6) {
                    out.nodes.push(row(
                        row_id("ms", &m.at),
                        "milestones",
                        &m.label,
                        &[
                            format!(
                                "{} ({})",
                                gantt::short_date(m.due),
                                countdown(m.due - today)
                            ),
                            m.light.word().into(),
                            m.reason.clone(),
                        ],
                        Some(m.light.color()),
                    ));
                }
            }
            "blockers" => {
                let list = blockers(doc);
                if list.is_empty() {
                    continue;
                }
                out.groups.push(table(
                    "blockers",
                    &format!("Open blockers ({})", list.len()),
                    &["Blocker", "Owner", "Where"],
                ));
                for b in list.iter().take(6) {
                    out.nodes.push(row(
                        row_id("bl", &b.at),
                        "blockers",
                        &b.label,
                        &[b.owner.clone(), b.place.clone()],
                        None,
                    ));
                }
            }
            "decisions" => {
                let list = decisions(doc);
                if list.is_empty() {
                    continue;
                }
                out.groups.push(table(
                    "decisions",
                    &format!("Decisions needed ({})", list.len()),
                    &["Decision", "Who", "By", "Where"],
                ));
                for x in list.iter().take(6) {
                    let late = x.due.as_str() != ""
                        && gantt::parse_date(&x.due).is_some_and(|z| z < today);
                    out.nodes.push(row(
                        row_id("de", &x.at),
                        "decisions",
                        &x.label,
                        &[x.owner.clone(), x.due.clone(), x.place.clone()],
                        late.then_some("red"),
                    ));
                }
            }
            "risks" => {
                let list = risks(doc);
                if list.is_empty() {
                    continue;
                }
                out.groups
                    .push(table("risks", "Top risks", &["Risk", "Score", "Owner"]));
                for r in list.iter().take(5) {
                    let c = if r.score >= 20.0 {
                        Some("red")
                    } else if r.score >= 12.0 {
                        Some("orange")
                    } else {
                        None
                    };
                    out.nodes.push(row(
                        row_id("ri", &r.at),
                        "risks",
                        &r.label,
                        &[format!("{:.0}", r.score), r.owner.clone()],
                        c,
                    ));
                }
            }
            "todos" => {
                let mut rows = Vec::new();
                for (path, d) in docs(doc) {
                    for g in d.groups.iter().filter(|g| g.todo) {
                        let (n, all) = crate::todo::progress(d, &g.id);
                        let at = At {
                            path: path.clone(),
                            id: g.id.clone(),
                        };
                        rows.push((
                            at,
                            g.display().to_string(),
                            format!("{n} of {all} done"),
                            n == all,
                        ));
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                out.groups
                    .push(table("todos", "Checklists", &["List", "Progress"]));
                for (at, label, prog, all) in rows {
                    out.nodes.push(row(
                        row_id("td", &at),
                        "todos",
                        &label,
                        &[prog],
                        all.then_some("green"),
                    ));
                }
            }
            "screens" | "records" => {
                let screens = *tile == "screens";
                let mut rows = Vec::new();
                for (path, d) in docs(doc) {
                    for g in &d.groups {
                        let what = if screens {
                            g.screen.map(|s| s.as_str().to_string())
                        } else {
                            g.adr.clone()
                        };
                        if let Some(w) = what {
                            rows.push((
                                At {
                                    path: path.clone(),
                                    id: g.id.clone(),
                                },
                                g.display().to_string(),
                                w,
                            ));
                        }
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                let (id, title, col) = if screens {
                    ("screens", "Screens", "Device")
                } else {
                    ("records", "Decision records", "Status")
                };
                out.groups.push(table(
                    id,
                    title,
                    &[if screens { "Screen" } else { "Decision" }, col],
                ));
                let most = if screens { 8 } else { 40 };
                for (at, label, w) in rows.into_iter().take(most) {
                    out.nodes
                        .push(row(row_id(&id[..2], &at), id, &label, &[w], None));
                }
            }
            "comments" => {
                let mut rows = Vec::new();
                for (path, d) in docs(doc) {
                    for c in d
                        .comments
                        .iter()
                        .filter(|c| c.status != crate::model::Status::Resolved)
                    {
                        let at = At {
                            path: path.clone(),
                            id: c.on.clone().unwrap_or_default(),
                        };
                        rows.push((at, c.text.clone(), c.author.clone().unwrap_or_default()));
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                out.groups.push(table(
                    "comments",
                    &format!("Open comments ({})", rows.len()),
                    &["Comment", "By"],
                ));
                for (i, (at, text, by)) in rows.into_iter().take(6).enumerate() {
                    let mut at = at;
                    at.id = if at.id.is_empty() {
                        format!("#{i}")
                    } else {
                        at.id
                    };
                    out.nodes.push(row(
                        row_id(&format!("co{i}"), &at),
                        "comments",
                        &text,
                        &[by],
                        None,
                    ));
                }
            }
            "money" if first_forecast(doc).is_some() => {
                for (path, d) in docs(doc) {
                    for g in d.groups.iter().filter(|g| g.forecast) {
                        let (mut chart, rows) = crate::forecast::as_chart(doc, d, g, today);
                        let id = format!("forecast-{}", g.id);
                        chart.id = id.clone();
                        out.groups.push(chart);
                        for (i, mut n) in rows.into_iter().enumerate() {
                            n.id = row_id(
                                &format!("fc{i}"),
                                &At {
                                    path: path.clone(),
                                    id: g.id.clone(),
                                },
                            );
                            n.group = Some(id.clone());
                            out.nodes.push(n);
                        }
                    }
                }
            }
            "money" => {
                // charts about money, as they are on the board
                for (path, d) in docs(doc) {
                    for g in d.groups.iter().filter(|g| g.chart.is_some()) {
                        let t = g.display();
                        let money = word_in(
                            t,
                            &[
                                "revenue", "mrr", "arr", "income", "sales", "margin", "money", "£",
                                "$", "€",
                            ],
                        ) || g
                            .unit
                            .as_deref()
                            .is_some_and(|u| word_in(u, &["£", "$", "€"]));
                        if !money || word_in(t, &["superseded", "obsolete", "old "]) {
                            continue;
                        }
                        let id = format!("money-{}", g.id);
                        let mut copy = g.clone();
                        copy.id = id.clone();
                        copy.parent = None;
                        copy.x = None;
                        copy.y = None;
                        out.groups.push(copy);
                        money_found = true;
                        for n in d
                            .nodes
                            .iter()
                            .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
                        {
                            let mut c = n.clone();
                            c.id = row_id(
                                "mo",
                                &At {
                                    path: path.clone(),
                                    id: n.id.clone(),
                                },
                            );
                            c.group = Some(id.clone());
                            out.nodes.push(c);
                        }
                    }
                }
                if !money_found {
                    out.groups
                        .push(table("money", "Money", &["Money", "What would show it"]));
                    let mut n = row("@dash/money/:".into(), "money", "Nothing about money on this board yet", &["Add a revenue chart (revenue now and by month), or tell the AI the price and how many customers to expect".into()], Some("gray"));
                    n.cells = vec![n.cells[0].clone()];
                    out.nodes.push(n);
                }
            }
            "scope" => {
                // the whole product: its mindmaps (those tagged for this
                // perspective, else all), two levels open, deeper ones folded
                let mut maps: Vec<(Vec<String>, &Document, &Node)> = Vec::new();
                for (path, d) in docs(doc) {
                    for n in d
                        .nodes
                        .iter()
                        .filter(|n| n.mindmap.is_some() && n.parent.is_none())
                    {
                        maps.push((path.clone(), d, n));
                    }
                }
                // the map most about this perspective: tagged for it, and the
                // fewest tags of the roles that have their own (a status or
                // pricing map is shared with Delivery and Business)
                let score = |n: &Node| {
                    let mine = n.tags.iter().any(|t| p.tags.contains(t)) as i32 * 10;
                    let shared = n
                        .tags
                        .iter()
                        .filter(|t| {
                            matches!(
                                t.as_str(),
                                "delivery" | "business" | "legal" | "team" | "finance"
                            )
                        })
                        .count() as i32;
                    mine - shared
                };
                if let Some((path, d, centre)) = maps.iter().max_by_key(|(_, _, n)| score(n)) {
                    copy_map(&mut out, d, path, &centre.id);
                }
            }
            "plan" => {
                // how the scope gets delivered: the plans, the boards, the dates
                let mut rows: Vec<(At, String, String, Option<&str>)> = Vec::new();
                for (path, d) in docs(doc) {
                    for g in d.groups.iter().filter(|g| g.gantt) {
                        let n = d
                            .nodes
                            .iter()
                            .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
                            .count();
                        rows.push((
                            At {
                                path: path.clone(),
                                id: g.id.clone(),
                            },
                            format!("{} ↗", g.display()),
                            format!("plan · {n} tasks"),
                            None,
                        ));
                    }
                    for g in d.groups.iter().filter(|g| g.kanban) {
                        let n = d
                            .nodes
                            .iter()
                            .filter(|n| ancestors(d, n).iter().any(|a| a.id == g.id))
                            .count();
                        rows.push((
                            At {
                                path: path.clone(),
                                id: g.id.clone(),
                            },
                            format!("{} ↗", g.display()),
                            format!("kanban · {n} cards"),
                            None,
                        ));
                    }
                }
                for m in ms.iter().take(6) {
                    rows.push((
                        m.at.clone(),
                        m.label.clone(),
                        format!(
                            "{} · {}, {}",
                            m.light.word(),
                            gantt::short_date(m.due),
                            countdown(m.due - today)
                        ),
                        Some(m.light.color()),
                    ));
                }
                if rows.is_empty() {
                    continue;
                }
                out.groups
                    .push(table("plan", "How we deliver it", &["", ""]));
                for (i, (at, label, what, color)) in rows.into_iter().enumerate() {
                    out.nodes.push(row(
                        row_id(&format!("pl{i}"), &at),
                        "plan",
                        &label,
                        &[what],
                        color,
                    ));
                }
                if let Some(g) = out.group_mut("plan") {
                    g.columns = vec!["Plan, board or milestone".into(), "".into()];
                }
            }
            "unrecorded" => {
                // decisions talked about but not written down as a record
                let list: Vec<Item> = decisions(doc)
                    .into_iter()
                    .filter(|x| {
                        doc.at(&x.at.path)
                            .and_then(|d| d.group(&x.at.id))
                            .is_none_or(|g| g.adr.is_none())
                    })
                    .collect();
                if list.is_empty() {
                    continue;
                }
                out.groups.push(table(
                    "unrecorded",
                    &format!("Decisions without a record ({})", list.len()),
                    &["Decision", "Where"],
                ));
                for x in list.iter().take(8) {
                    out.nodes.push(row(
                        row_id("un", &x.at),
                        "unrecorded",
                        &x.label,
                        &[x.place.clone()],
                        Some("orange"),
                    ));
                }
            }
            "burndown" => {
                let Some(m) = next else { continue };
                if let Some(chart) = burndown(doc, m, today) {
                    out.groups.push(chart.0);
                    out.nodes.extend(chart.1);
                }
            }
            "kanban" => {
                let mut rows = Vec::new();
                for (path, d) in docs(doc) {
                    for k in d.groups.iter().filter(|g| g.kanban) {
                        let counts: Vec<String> = d
                            .groups
                            .iter()
                            .filter(|c| c.parent.as_deref() == Some(k.id.as_str()))
                            .map(|c| {
                                let n = d
                                    .nodes
                                    .iter()
                                    .filter(|n| n.group.as_deref() == Some(c.id.as_str()))
                                    .count();
                                format!("{n} {}", c.display().to_lowercase())
                            })
                            .collect();
                        rows.push((
                            At {
                                path: path.clone(),
                                id: k.id.clone(),
                            },
                            k.display().to_string(),
                            counts.join(" · "),
                        ));
                    }
                }
                if rows.is_empty() {
                    continue;
                }
                out.groups
                    .push(table("kanban", "Kanban boards", &["Board", "Cards"]));
                for (at, label, counts) in rows {
                    out.nodes.push(row(
                        row_id("kb", &at),
                        "kanban",
                        &format!("{label} ↗"),
                        &[counts],
                        None,
                    ));
                }
            }
            "gantt" => {
                // the plan the current delivery is on, when it waits on more than one thing
                let Some(m) = next.filter(|m| m.work.len() > 1) else {
                    continue;
                };
                if let Some(d) = doc.at(&m.at.path) {
                    copy_block(&mut out, d, &m.at.path, &m.chart, "gantt", "ga");
                }
            }
            "matrix" => {
                for (path, d) in docs(doc) {
                    for g in d.groups.iter().filter(|g| g.risks) {
                        copy_block(&mut out, d, &path, &g.id, &format!("matrix-{}", g.id), "ri");
                    }
                }
            }
            "since" => {
                let Some(old) = before else { continue };
                let list = since(old, doc);
                if list.is_empty() {
                    continue;
                }
                out.groups.push(table(
                    "since",
                    &format!("Changed since ({})", list.len()),
                    &["What"],
                ));
                for (at, what) in list.into_iter().take(8) {
                    out.nodes
                        .push(row(row_id("si", &at), "since", &what, &[], None));
                }
            }
            _ => {}
        }
    }
    // the comments on what the summary shows, on its items (a pin opens the thread on the board)
    let mut copies = Vec::new();
    for (path, d) in docs(doc) {
        for (i, c) in d.comments.iter().enumerate() {
            let Some(on) = c.on.as_deref() else { continue };
            let shown = out
                .nodes
                .iter()
                .find(|n| target(&n.id).is_some_and(|(p, id)| p == path && id == on));
            if let Some(n) = shown {
                let mut copy = c.clone();
                copy.id = row_id(
                    &format!("cm{i}"),
                    &At {
                        path: path.clone(),
                        id: on.to_string(),
                    },
                );
                copy.on = Some(n.id.clone());
                copies.push(copy);
            }
        }
    }
    out.comments.extend(copies);
    drop_empty_columns(&mut out);
    arrange(&mut out);
    out
}

/// The first forecast on the board (in the top board or a canvas).
fn first_forecast(doc: &Document) -> Option<(&Document, &Group)> {
    docs(doc)
        .into_iter()
        .find_map(|(_, d)| d.groups.iter().find(|g| g.forecast).map(|g| (d, g)))
}

/// Mindmap `centre` of `d`, its centre and two levels below open (what is
/// deeper folded), with dashboard ids so a click opens the topic.
fn copy_map(out: &mut Document, d: &Document, path: &[String], centre: &str) {
    let new_id = |old: &str| {
        row_id(
            "mm",
            &At {
                path: path.to_vec(),
                id: old.to_string(),
            },
        )
    };
    let mut level = vec![(centre.to_string(), 0usize)];
    while let Some((id, depth)) = level.pop() {
        let Some(n) = d.node(&id) else { continue };
        let kids: Vec<&Node> = d
            .nodes
            .iter()
            .filter(|k| k.parent.as_deref() == Some(id.as_str()))
            .collect();
        let mut c = n.clone();
        c.id = new_id(&n.id);
        c.parent = n.parent.as_deref().map(new_id);
        c.group = None;
        c.x = None;
        c.y = None;
        c.collapsed = n.collapsed || (depth >= 2 && !kids.is_empty());
        out.nodes.push(c);
        if depth < 2 && !n.collapsed {
            for k in kids.iter().rev() {
                level.push((k.id.clone(), depth + 1));
            }
        }
    }
    // the topics keep their order: the walk went depth first
    let start = out
        .nodes
        .iter()
        .position(|n| n.id == new_id(centre))
        .unwrap_or(0);
    let mut part: Vec<Node> = out.nodes.split_off(start);
    let order: Vec<String> = d.nodes.iter().map(|n| new_id(&n.id)).collect();
    part.sort_by_key(|n| order.iter().position(|o| *o == n.id).unwrap_or(usize::MAX));
    out.nodes.extend(part);
}

/// A block of the board, as it is, under id `id`: its items get dashboard
/// ids (tile `tile`), so a click opens them, and keep their links to each other.
fn copy_block(out: &mut Document, d: &Document, path: &[String], gid: &str, id: &str, tile: &str) {
    let Some(g) = d.group(gid) else { return };
    let mut copy = g.clone();
    copy.id = id.to_string();
    copy.parent = None;
    copy.x = None;
    copy.y = None;
    let new_id = |old: &str| {
        row_id(
            tile,
            &At {
                path: path.to_vec(),
                id: old.to_string(),
            },
        )
    };
    out.groups.push(copy);
    for n in d.nodes.iter().filter(|n| n.group.as_deref() == Some(gid)) {
        let mut c = n.clone();
        c.id = new_id(&n.id);
        c.group = Some(id.to_string());
        c.after = n.after.iter().map(|a| new_id(a)).collect();
        c.x = None;
        c.y = None;
        out.nodes.push(c);
    }
}

/// The work left before milestone `m`, in days of work: as planned (each
/// task burning down over its planned days) and, from today, as forecast
/// (what is left now, done by the latest forecast end). A chart on the
/// board called a burndown is shown instead, when there is one.
fn burndown(doc: &Document, m: &Milestone, today: i64) -> Option<(Group, Vec<Node>)> {
    for (path, d) in docs(doc) {
        if let Some(g) = d.groups.iter().find(|g| {
            g.chart.is_some()
                && word_in(
                    &format!("{} {}", g.id, g.display()),
                    &["burndown", "burn-down", "burn down"],
                )
        }) {
            let mut tmp = Document::default();
            copy_block(&mut tmp, d, &path, &g.id, "burndown", "bu");
            let group = tmp.groups.pop()?;
            return Some((group, tmp.nodes));
        }
    }
    let d = doc.at(&m.at.path)?;
    let tracks = gantt::track(d, &m.chart, today);
    let day = |s: &str| gantt::parse_date(s).unwrap_or(today);
    let work: Vec<&gantt::Track> = tracks.iter().filter(|t| m.work.contains(&t.id)).collect();
    if work.is_empty() {
        return None;
    }
    let from = work.iter().map(|t| day(&t.start)).min()?;
    let fend = work.iter().map(|t| day(&t.forecast_end)).max()?.max(today);
    let to = m.due.max(fend);
    let planned = |x: i64| -> f64 {
        work.iter()
            .map(|t| {
                let (s, e) = (day(&t.start), day(&t.end) + 1);
                let left = ((e - x) as f64 / (e - s).max(1) as f64).clamp(0.0, 1.0);
                t.days * left
            })
            .sum()
    };
    let now: f64 = work
        .iter()
        .map(|t| t.days * (1.0 - t.done.unwrap_or(0.0) / 100.0).max(0.0))
        .sum();
    let forecast = |x: i64| -> Option<f64> {
        (x >= today)
            .then(|| now * ((fend - x) as f64 / (fend - today).max(1) as f64).clamp(0.0, 1.0))
    };
    let mut days: Vec<i64> = (0..=8).map(|k| from + (to - from) * k / 8).collect();
    days.extend([today, m.due]);
    days.retain(|x| *x >= from && *x <= to);
    days.sort();
    days.dedup();
    let mut g = Group {
        id: "burndown".into(),
        label: format!(
            "Work left before {} (due {})",
            m.label,
            gantt::short_date(m.due)
        ),
        chart: Some(crate::model::ChartKind::Line),
        series: vec!["Planned".into(), "Forecast".into()],
        unit: Some("d".into()),
        ..Default::default()
    };
    g.w = Some(520.0);
    let rows = days
        .into_iter()
        .map(|x| {
            let mut n = Node::new(&format!("@dash/bu/:{x}"));
            n.label = if x == m.due {
                format!("{} due", gantt::short_date(x))
            } else if x == today {
                "today".into()
            } else {
                gantt::short_date(x)
            };
            n.group = Some("burndown".into());
            n.values = vec![
                Some((planned(x) * 10.0).round() / 10.0),
                forecast(x).map(|v| (v * 10.0).round() / 10.0),
            ];
            n
        })
        .collect();
    Some((g, rows))
}

/// A column nobody filled in (no owners, no dates) is left out.
fn drop_empty_columns(d: &mut Document) {
    for g in d.groups.iter_mut().filter(|g| g.table) {
        let rows: Vec<&mut Node> = d
            .nodes
            .iter_mut()
            .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
            .collect();
        let mut keep: Vec<bool> = (0..g.columns.len()).map(|c| c == 0).collect();
        for r in &rows {
            for (k, cell) in r.cells.iter().enumerate() {
                if !cell.trim().is_empty() {
                    if let Some(x) = keep.get_mut(k + 1) {
                        *x = true;
                    }
                }
            }
        }
        if keep.iter().all(|k| *k) {
            continue;
        }
        g.columns = g
            .columns
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(c, _)| c.clone())
            .collect();
        for r in rows {
            r.cells = r
                .cells
                .iter()
                .enumerate()
                .filter(|(k, _)| keep.get(k + 1).copied().unwrap_or(false))
                .map(|(_, c)| c.clone())
                .collect();
        }
    }
}

/// The headline across the top, milestones under it, the other tiles in two
/// columns, each going under the shorter one.
fn arrange(d: &mut Document) {
    const GAP: f64 = 28.0;
    // a block's frame around what it draws: its title and padding
    let size = |d: &Document, g: &Group| {
        let p = crate::charts::plot_any(d, g);
        (p.w + 40.0, p.h + 76.0)
    };
    let sizes: Vec<(String, f64, f64)> = d
        .groups
        .iter()
        .map(|g| {
            let (w, h) = size(d, g);
            (g.id.clone(), w, h)
        })
        .collect();
    let first = sizes
        .first()
        .filter(|(id, _, _)| id == "milestones")
        .cloned();
    let rest: Vec<&(String, f64, f64)> = sizes
        .iter()
        .filter(|(id, _, _)| first.as_ref().is_none_or(|f| f.0 != *id))
        .collect();
    let col_w = rest.iter().map(|(_, w, _)| *w).fold(0.0_f64, f64::max);
    let width = first
        .as_ref()
        .map(|f| f.1)
        .unwrap_or(0.0)
        .max(2.0 * col_w + GAP)
        .max(720.0);
    let head_h = 72.0;
    if let Some(h) = d.nodes.first_mut() {
        h.x = Some(0.0);
        h.y = Some(0.0);
        h.w = Some(width);
    }
    let mut y = head_h + GAP;
    let place = |d: &mut Document, id: &str, x: f64, y: f64| {
        if let Some(g) = d.group_mut(id) {
            g.x = Some(x);
            g.y = Some(y);
        }
    };
    if let Some((id, _, h)) = &first {
        place(d, id, 0.0, y);
        y += h + GAP;
    }
    let mut cols = [y, y];
    let xs = [0.0, (width - GAP) / 2.0 + GAP];
    for (id, w, h) in rest {
        // a block wider than a column (a gantt chart) takes the whole width
        if *w > (width - GAP) / 2.0 {
            let y = cols[0].max(cols[1]);
            place(d, id, 0.0, y);
            cols = [y + h + GAP, y + h + GAP];
            continue;
        }
        let c = if cols[0] <= cols[1] { 0 } else { 1 };
        place(d, id, xs[c], cols[c]);
        cols[c] += h + GAP;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOARD: &str = "title: Launch\ntype: project\n\n\
        canvas plan: Plan {\n\
        \x20 gantt road: Roadmap [start: 2026-10-01] {\n\
        \x20   Build [2026-10-01, 3w, done: 30%, owner: ana]\n\
        \x20   Beta [2026-10-22, milestone]\n\
        \x20   Docs [2026-10-23, 1w]\n\
        \x20   Launch [2026-11-20, milestone]\n\
        \x20 }\n\
        \x20 risks r: Risks {\n\
        \x20   Vendor review slips [p: 4, i: 5, owner: sam]\n\
        \x20   Scope creep [p: 2, i: 2]\n\
        \x20 }\n\
        \x20 kanban k: Blockers {\n\
        \x20   column open: Open {\n\
        \x20     key: No signing key [owner: ana]\n\
        \x20   }\n\
        \x20   column done: Solved {\n\
        \x20     old: Old thing\n\
        \x20   }\n\
        \x20 }\n\
        }\n\
        group asks: What we need from you {\n\
        \x20 price: Set the price [owner: cfo, date: 2026-10-25]\n\
        }\n";

    #[test]
    fn delivery_answers_are_we_on_plan() {
        let doc = crate::dsl::parse(BOARD).unwrap();
        let today = gantt::parse_date("2026-10-15").unwrap();
        let ms = milestones(&doc, today);
        assert_eq!(
            ms.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(),
            vec!["Beta", "Launch"]
        );
        // the work before the beta is 30% done with 6 of 21 days left: it ends after it
        assert_eq!(ms[0].light, Light::Red);
        assert!(
            ms[0].reason.starts_with("Build forecast to end"),
            "{}",
            ms[0].reason
        );
        assert!(
            ms[0].reason.contains("1 open blocker") && ms[0].reason.contains("1 high risk"),
            "{}",
            ms[0].reason
        );
        assert_eq!(ms[1].light, Light::Green);
        assert_eq!(
            blockers(&doc)
                .iter()
                .map(|b| b.label.as_str())
                .collect::<Vec<_>>(),
            vec!["No signing key"]
        );
        assert_eq!(decisions(&doc)[0].label, "Set the price");
        assert_eq!(risks(&doc)[0].score, 20.0);

        let p = crate::perspectives::all(&doc)
            .into_iter()
            .find(|p| p.id == "delivery")
            .unwrap();
        let dash = build(&doc, &p, today, None);
        // what is being delivered now, its light and why; the work left; the
        // boards; the risks (the plan only when it waits on more than one thing)
        assert_eq!(dash.nodes[0].label, "Now delivering: Beta");
        assert_eq!(dash.nodes[0].color.as_deref(), Some("red"));
        let detail = dash.nodes[0].detail.clone().unwrap();
        assert!(
            detail.starts_with("Off track · due Oct 22, in 7 days · Build forecast"),
            "{detail}"
        );
        let blocks: Vec<&str> = dash.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(blocks, vec!["burndown", "kanban", "matrix-r"]);
        let rows: Vec<&Node> = dash
            .nodes
            .iter()
            .filter(|n| n.group.as_deref() == Some("burndown"))
            .collect();
        assert_eq!(
            rows.first().unwrap().values[0],
            Some(21.0),
            "all of Build left at its start"
        );
        assert_eq!(rows.last().unwrap().values, vec![Some(0.0), Some(0.0)]);
        let board = dash
            .nodes
            .iter()
            .find(|n| n.group.as_deref() == Some("kanban"))
            .unwrap();
        assert_eq!(
            target(&board.id),
            Some((vec!["plan".to_string()], "k".to_string()))
        );
        let risk = dash
            .nodes
            .iter()
            .find(|n| n.label == "Vendor review slips")
            .unwrap();
        assert_eq!(
            target(&risk.id),
            Some((vec!["plan".to_string()], "vendor-review-slips".to_string()))
        );
        // with two things to wait on, the plan comes too
        let two = crate::dsl::parse(
            &BOARD.replace("    Docs [2026-10-23, 1w]\n", "    Docs [2026-10-10, 1w]\n"),
        )
        .unwrap();
        let dash2 = build(&two, &p, today, None);
        assert!(dash2.groups.iter().any(|g| g.id == "gantt" && g.gantt));
        // it renders
        let svg = crate::render::render(&dash, &Default::default()).svg;
        assert!(svg.contains("Now delivering"));
    }

    #[test]
    fn product_business_and_technical_lead_with_their_question() {
        let src = format!(
            "{BOARD}\nmindmap status: Where we are [tags: delivery business] {{\n  Built\n}}\n\
             mindmap what: Launch app [tags: product design] {{\n  Accounts\n    Sign up\n      With Google\n  Billing\n}}\n\
             forecast revenue: Revenue [unit: £] {{\n  now: 1k\n  Launch [adds: 4k, ramp: 2]\n}}\n\
             adr db: Use Postgres [accepted] {{\n  decision: One database.\n}}\n"
        );
        let doc = crate::dsl::parse(&src).unwrap();
        let today = gantt::parse_date("2026-10-15").unwrap();
        let get = |id: &str| {
            crate::perspectives::all(&doc)
                .into_iter()
                .find(|p| p.id == id)
                .unwrap()
        };

        // Product: the whole product (the map most about it), and how it gets delivered
        let d = build(&doc, &get("product"), today, None);
        assert_eq!(d.nodes[0].label, "What we are building: Launch");
        assert!(d
            .nodes
            .iter()
            .any(|n| n.label == "Launch app" && n.mindmap.is_some()));
        assert!(!d.nodes.iter().any(|n| n.label == "Where we are"));
        // two levels open, what is deeper folded
        let signup = d.nodes.iter().find(|n| n.label == "Sign up").unwrap();
        assert!(signup.collapsed);
        let plan: Vec<String> = d
            .nodes
            .iter()
            .filter(|n| n.group.as_deref() == Some("plan"))
            .map(|n| n.label.clone())
            .collect();
        assert_eq!(plan, vec!["Roadmap ↗", "Blockers ↗", "Beta", "Launch"]);

        // Business: money now, where it gets to, what the slips cost
        let d = build(&doc, &get("business"), today, None);
        assert!(
            d.nodes[0]
                .label
                .starts_with("Revenue: £1k a month now → £5k by "),
            "{}",
            d.nodes[0].label
        );
        assert!(d
            .groups
            .iter()
            .any(|g| g.chart.is_some() && g.id == "forecast-revenue"));

        // Technical: every decision record, and the decisions without one
        let d = build(&doc, &get("tech"), today, None);
        assert_eq!(
            d.nodes[0].label,
            "Architecture decisions: 1 recorded, 1 not written down yet"
        );
        assert!(d
            .nodes
            .iter()
            .any(|n| n.group.as_deref() == Some("records") && n.label == "Use Postgres"));
        assert!(d
            .nodes
            .iter()
            .any(|n| n.group.as_deref() == Some("unrecorded") && n.label == "Set the price"));
    }

    #[test]
    fn what_changed_since_an_earlier_version() {
        let old = crate::dsl::parse(BOARD).unwrap();
        let new = crate::dsl::parse(
            &BOARD
                .replace(
                    "Build [2026-10-01, 3w, done: 30%",
                    "Build [2026-10-01, 4w, done: 30%",
                )
                .replace("[p: 2, i: 2]", "[p: 4, i: 2]"),
        )
        .unwrap();
        let s: Vec<String> = since(&old, &new).into_iter().map(|(_, w)| w).collect();
        assert!(
            s.contains(&"Moved: Build".to_string())
                && s.contains(&"Re-rated: Scope creep".to_string()),
            "{s:?}"
        );
    }
}
