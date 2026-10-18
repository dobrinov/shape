//! Wireframe screens: a group drawn as a phone, a tablet or a browser
//! window, its nodes components from top to bottom (a header, fields,
//! buttons, lists…). A component is an ordinary node, so an arrow from one
//! (`login.submit -> home`) draws a flow between screens.
//!
//! ```text
//! screen login: Sign in [mobile] {
//!   header: Acme
//!   title: Welcome back
//!   input email: Email
//!   password: Password
//!   checkbox: Remember me
//!   button submit: Sign in [blue]
//!   link forgot: Forgot password?
//! }
//! ```
//!
//! Items of lists, tabs, menus and the like are written in the label,
//! separated by `|` (`tabs: Home | Search | Profile`).
use crate::charts::{Mark, Paint, Plot};
use crate::layout::Rect;
use crate::model::{Device, Document, Group, Node, Widget};
use crate::text;

const PAD: f64 = 16.0;
const GAP: f64 = 12.0;

/// The screen a node is a component of.
pub fn screen_of<'a>(doc: &'a Document, n: &Node) -> Option<&'a Group> {
    n.group
        .as_deref()
        .and_then(|g| doc.group(g))
        .filter(|g| g.screen.is_some())
}

/// The items of a component (`Home | Search | Profile`).
pub fn items(label: &str) -> Vec<String> {
    label
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// (width, minimum height, top chrome) of a device's frame.
fn frame(d: Device) -> (f64, f64, f64) {
    match d {
        Device::Mobile => (360.0, 640.0, 40.0),
        Device::Tablet => (600.0, 760.0, 32.0),
        Device::Web => (1024.0, 600.0, 40.0),
    }
}

struct Draw {
    marks: Vec<Mark>,
}

impl Draw {
    fn fill(&mut self, x: f64, y: f64, w: f64, h: f64, paint: Paint, round: f64) {
        self.marks.push(Mark::Rect {
            r: Rect { x, y, w, h },
            paint,
            round,
        });
    }
    /// An outlined box: a border (`line`) round a fill.
    fn boxed(&mut self, x: f64, y: f64, w: f64, h: f64, round: f64, fill: Paint) {
        self.fill(x, y, w, h, Paint::Grid, round);
        self.fill(
            x + 1.0,
            y + 1.0,
            w - 2.0,
            h - 2.0,
            fill,
            (round - 1.0).max(0.0),
        );
    }
    fn text(&mut self, x: f64, y: f64, t: &str, size: f64, paint: Paint, bold: bool) {
        self.text_at(x, y, t, size, paint, bold, "start");
    }
    #[allow(clippy::too_many_arguments)]
    fn text_at(
        &mut self,
        x: f64,
        y: f64,
        t: &str,
        size: f64,
        paint: Paint,
        bold: bool,
        anchor: &'static str,
    ) {
        self.marks.push(Mark::Text {
            x,
            y,
            text: t.to_string(),
            size,
            anchor,
            paint,
            bold,
        });
    }
    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, paint: Paint, width: f64) {
        self.marks.push(Mark::Line {
            x1,
            y1,
            x2,
            y2,
            paint,
            width,
        });
    }
    fn path(&mut self, d: String, stroke: Option<Paint>, fill: Option<Paint>, width: f64) {
        self.marks.push(Mark::Path {
            d,
            stroke,
            fill,
            width,
            opacity: 1.0,
        });
    }
    fn circle(&mut self, cx: f64, cy: f64, r: f64, paint: Paint) {
        self.marks.push(Mark::Circle { cx, cy, r, paint });
    }
    /// An image placeholder: a box with a cross through it.
    fn placeholder(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.boxed(x, y, w, h, 6.0, Paint::Header);
        self.line(x + 6.0, y + 6.0, x + w - 6.0, y + h - 6.0, Paint::Grid, 1.0);
        self.line(x + w - 6.0, y + 6.0, x + 6.0, y + h - 6.0, Paint::Grid, 1.0);
    }
}

/// Lines of `t` wrapped to `w` at `size`.
fn wrap(t: &str, size: f64, w: f64) -> Vec<String> {
    if t.is_empty() {
        return Vec::new();
    }
    text::wrap(t, size, w)
}

/// The screen drawn: its device's frame, then each component in turn; each
/// node's box is where its component is (for selection and arrows).
pub fn plot(doc: &Document, g: &Group) -> Plot {
    let device = g.screen.unwrap_or_default();
    let (fw, min_h, chrome) = frame(device);
    let parts: Vec<&Node> = doc
        .nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(g.id.as_str()))
        .collect();
    let mut d = Draw { marks: Vec::new() };
    let mut rows: Vec<(String, Rect)> = Vec::new();
    // a sidebar takes the left of a wide screen; a tab bar and a floating
    // button sit at the bottom whatever their place in the list
    let sidebar = parts
        .iter()
        .find(|n| n.widget == Some(Widget::Sidebar) && device != Device::Mobile);
    let side_w = if sidebar.is_some() { 200.0 } else { 0.0 };
    let x0 = side_w + PAD;
    let cw = fw - side_w - 2.0 * PAD;
    let mut y = chrome + PAD;
    let mut body = Draw { marks: Vec::new() };
    let accent = |n: &Node| n.color.clone().unwrap_or_else(|| "blue".into());
    for n in &parts {
        let kind = n.widget.unwrap_or_default();
        let label = n.label.as_str();
        let its = items(label);
        let start = y;
        // full-width bars (header, navbar) go edge to edge
        let (bx, bw) = (x0, cw);
        match kind {
            Widget::Sidebar if device != Device::Mobile => continue,
            Widget::Tabbar | Widget::Fab => continue,
            Widget::Header => {
                let top = y - PAD;
                body.fill(side_w, top, fw - side_w, 52.0, Paint::Header, 0.0);
                body.text_at(
                    side_w + 18.0,
                    top + 32.0,
                    "‹",
                    22.0,
                    Paint::Fg,
                    false,
                    "start",
                );
                body.text_at(
                    side_w + (fw - side_w) / 2.0,
                    top + 32.0,
                    label,
                    17.0,
                    Paint::Fg,
                    true,
                    "middle",
                );
                body.text_at(fw - 18.0, top + 31.0, "⋯", 18.0, Paint::Muted, true, "end");
                rows.push((
                    n.id.clone(),
                    Rect {
                        x: side_w,
                        y: top,
                        w: fw - side_w,
                        h: 52.0,
                    },
                ));
                y = top + 52.0 + PAD;
                continue;
            }
            Widget::Navbar => {
                let top = y - PAD;
                body.fill(side_w, top, fw - side_w, 56.0, Paint::Header, 0.0);
                let (first, rest) = its
                    .split_first()
                    .map(|(f, r)| (f.as_str(), r))
                    .unwrap_or(("", &[][..]));
                body.text(x0, top + 34.0, first, 17.0, Paint::Fg, true);
                if device == Device::Mobile {
                    body.text_at(fw - 18.0, top + 34.0, "☰", 18.0, Paint::Fg, false, "end");
                } else {
                    let mut x = fw - PAD;
                    for (k, it) in rest.iter().enumerate().rev() {
                        let w = text::width(it, 14.0);
                        if k == rest.len() - 1 && rest.len() > 1 {
                            // the last one is a button (sign in, try it)
                            let bw = w + 24.0;
                            body.fill(x - bw, top + 14.0, bw, 28.0, Paint::Color(accent(n)), 6.0);
                            body.text_at(
                                x - bw / 2.0,
                                top + 33.0,
                                it,
                                14.0,
                                Paint::Bg,
                                true,
                                "middle",
                            );
                            x -= bw + 20.0;
                        } else {
                            body.text_at(x, top + 33.0, it, 14.0, Paint::Muted, false, "end");
                            x -= w + 22.0;
                        }
                    }
                }
                rows.push((
                    n.id.clone(),
                    Rect {
                        x: side_w,
                        y: top,
                        w: fw - side_w,
                        h: 56.0,
                    },
                ));
                y = top + 56.0 + PAD;
                continue;
            }
            Widget::Title => {
                let lines = wrap(label, 22.0, bw);
                for (k, l) in lines.iter().enumerate() {
                    body.text(bx, y + 22.0 + k as f64 * 28.0, l, 22.0, Paint::Fg, true);
                }
                y += lines.len().max(1) as f64 * 28.0 + 2.0;
            }
            Widget::Subtitle | Widget::Text => {
                let size = if kind == Widget::Subtitle { 16.0 } else { 14.0 };
                let paint = if kind == Widget::Subtitle {
                    Paint::Fg
                } else {
                    Paint::Muted
                };
                let lines = wrap(label, size, bw);
                for (k, l) in lines.iter().enumerate() {
                    body.text(
                        bx,
                        y + size + k as f64 * (size + 6.0),
                        l,
                        size,
                        paint.clone(),
                        false,
                    );
                }
                y += lines.len().max(1) as f64 * (size + 6.0);
            }
            Widget::Button => {
                let filled = n.color.is_some();
                if filled {
                    body.fill(bx, y, bw, 44.0, Paint::Color(accent(n)), 8.0);
                    body.text_at(
                        bx + bw / 2.0,
                        y + 27.0,
                        label,
                        15.0,
                        Paint::Bg,
                        true,
                        "middle",
                    );
                } else {
                    body.boxed(bx, y, bw, 44.0, 8.0, Paint::Bg);
                    body.text_at(
                        bx + bw / 2.0,
                        y + 27.0,
                        label,
                        15.0,
                        Paint::Fg,
                        true,
                        "middle",
                    );
                }
                y += 44.0;
            }
            Widget::Buttons => {
                let k = its.len().max(1) as f64;
                let w = (bw - (k - 1.0) * 10.0) / k;
                for (i, it) in its.iter().enumerate() {
                    let x = bx + i as f64 * (w + 10.0);
                    if i + 1 == its.len() && its.len() > 1 {
                        body.fill(x, y, w, 44.0, Paint::Color(accent(n)), 8.0);
                        body.text_at(x + w / 2.0, y + 27.0, it, 15.0, Paint::Bg, true, "middle");
                    } else {
                        body.boxed(x, y, w, 44.0, 8.0, Paint::Bg);
                        body.text_at(x + w / 2.0, y + 27.0, it, 15.0, Paint::Fg, true, "middle");
                    }
                }
                y += 44.0;
            }
            Widget::Link => {
                body.text(bx, y + 15.0, label, 14.0, Paint::Color(accent(n)), true);
                let w = text::width(label, 14.0);
                body.line(bx, y + 18.5, bx + w, y + 18.5, Paint::Color(accent(n)), 1.0);
                y += 22.0;
            }
            Widget::Input
            | Widget::Password
            | Widget::Search
            | Widget::Select
            | Widget::Date
            | Widget::Textarea => {
                let (field, hint) = match kind {
                    Widget::Search => (
                        None,
                        format!("⌕  {}", if label.is_empty() { "Search" } else { label }),
                    ),
                    Widget::Password => (Some(label), "••••••••".to_string()),
                    Widget::Date => (Some(label), "dd / mm / yyyy".to_string()),
                    _ => (Some(label), String::new()),
                };
                if let Some(f) = field.filter(|f| !f.is_empty()) {
                    body.text(bx, y + 12.0, f, 12.5, Paint::Muted, true);
                    y += 18.0;
                }
                let h = if kind == Widget::Textarea { 96.0 } else { 42.0 };
                let round = if kind == Widget::Search { 21.0 } else { 8.0 };
                body.boxed(bx, y, bw, h, round, Paint::Bg);
                if !hint.is_empty() {
                    body.text(bx + 14.0, y + 26.0, &hint, 14.0, Paint::Muted, false);
                }
                if kind == Widget::Select {
                    body.text_at(
                        bx + bw - 14.0,
                        y + 26.0,
                        "▾",
                        14.0,
                        Paint::Muted,
                        false,
                        "end",
                    );
                }
                if kind == Widget::Date {
                    body.text_at(
                        bx + bw - 14.0,
                        y + 26.0,
                        "▦",
                        14.0,
                        Paint::Muted,
                        false,
                        "end",
                    );
                }
                y += h;
            }
            Widget::Checkbox => {
                body.boxed(bx, y + 2.0, 18.0, 18.0, 4.0, Paint::Bg);
                body.path(
                    format!("M{} {}l4 4l7 -8", bx + 4.5, y + 11.0),
                    Some(Paint::Fg),
                    None,
                    2.0,
                );
                body.text(bx + 28.0, y + 16.0, label, 14.0, Paint::Fg, false);
                y += 22.0;
            }
            Widget::Radio => {
                let opts = if its.is_empty() {
                    vec![String::new()]
                } else {
                    its.clone()
                };
                for (i, it) in opts.iter().enumerate() {
                    let cy = y + 11.0 + i as f64 * 28.0;
                    body.circle(bx + 9.0, cy, 9.0, Paint::Grid);
                    body.circle(bx + 9.0, cy, 8.0, Paint::Bg);
                    if i == 0 {
                        body.circle(bx + 9.0, cy, 4.5, Paint::Fg);
                    }
                    body.text(bx + 28.0, cy + 5.0, it, 14.0, Paint::Fg, false);
                }
                y += opts.len() as f64 * 28.0 - 6.0;
            }
            Widget::Toggle => {
                body.text(bx, y + 17.0, label, 14.0, Paint::Fg, false);
                body.fill(
                    bx + bw - 44.0,
                    y + 2.0,
                    44.0,
                    24.0,
                    Paint::Color(accent(n)),
                    12.0,
                );
                body.circle(bx + bw - 14.0, y + 14.0, 9.0, Paint::Bg);
                y += 28.0;
            }
            Widget::Slider => {
                if !label.is_empty() {
                    body.text(bx, y + 12.0, label, 12.5, Paint::Muted, true);
                    y += 20.0;
                }
                body.fill(bx, y + 8.0, bw, 4.0, Paint::Grid, 2.0);
                body.fill(bx, y + 8.0, bw * 0.6, 4.0, Paint::Color(accent(n)), 2.0);
                body.circle(bx + bw * 0.6, y + 10.0, 9.0, Paint::Color(accent(n)));
                y += 22.0;
            }
            Widget::Image | Widget::Video | Widget::Map | Widget::Chart => {
                let h = (bw * 0.56).min(220.0);
                match kind {
                    Widget::Image => body.placeholder(bx, y, bw, h),
                    Widget::Video => {
                        body.fill(bx, y, bw, h, Paint::Fg, 8.0);
                        let (cx, cy) = (bx + bw / 2.0, y + h / 2.0);
                        body.circle(cx, cy, 24.0, Paint::Bg);
                        body.path(
                            format!("M{} {}l14 9l-14 9z", cx - 5.0, cy - 9.0),
                            None,
                            Some(Paint::Fg),
                            0.0,
                        );
                    }
                    Widget::Map => {
                        body.boxed(bx, y, bw, h, 8.0, Paint::Header);
                        for k in 1..5 {
                            let f = k as f64 / 5.0;
                            body.line(
                                bx + bw * f,
                                y + 1.0,
                                bx + bw * f - 30.0,
                                y + h - 1.0,
                                Paint::Grid,
                                1.0,
                            );
                            body.line(
                                bx + 1.0,
                                y + h * f,
                                bx + bw - 1.0,
                                y + h * f + 12.0,
                                Paint::Grid,
                                1.0,
                            );
                        }
                        let (px, py) = (bx + bw * 0.55, y + h * 0.45);
                        body.path(
                            format!("M{px} {py}c-9 -12 -9 -24 0 -24c9 0 9 12 0 24z"),
                            None,
                            Some(Paint::Color(accent(n))),
                            0.0,
                        );
                        body.circle(px, py - 15.0, 3.5, Paint::Bg);
                    }
                    _ => {
                        body.boxed(bx, y, bw, h, 8.0, Paint::Bg);
                        let bars = [0.45, 0.7, 0.55, 0.85, 0.6, 0.95];
                        let slot = (bw - 32.0) / bars.len() as f64;
                        for (k, f) in bars.iter().enumerate() {
                            let bh = (h - 36.0) * f;
                            body.fill(
                                bx + 16.0 + k as f64 * slot + slot * 0.2,
                                y + h - 16.0 - bh,
                                slot * 0.6,
                                bh,
                                Paint::Color(accent(n)),
                                3.0,
                            );
                        }
                    }
                }
                if !label.is_empty() && kind != Widget::Chart {
                    body.text_at(
                        bx + bw / 2.0,
                        y + h + 18.0,
                        label,
                        12.5,
                        Paint::Muted,
                        false,
                        "middle",
                    );
                    y += 22.0;
                } else if kind == Widget::Chart && !label.is_empty() {
                    body.text(bx + 12.0, y + 20.0, label, 13.0, Paint::Fg, true);
                }
                y += h;
            }
            Widget::Avatar => {
                body.circle(bx + 22.0, y + 22.0, 22.0, Paint::Header);
                body.circle(bx + 22.0, y + 17.0, 8.0, Paint::Grid);
                body.path(
                    format!("M{} {}a13 10 0 0 1 26 0z", bx + 9.0, y + 38.0),
                    None,
                    Some(Paint::Grid),
                    0.0,
                );
                let (name, more) = its
                    .split_first()
                    .map(|(f, r)| (f.clone(), r.join(" · ")))
                    .unwrap_or_default();
                body.text(
                    bx + 56.0,
                    y + if more.is_empty() { 27.0 } else { 19.0 },
                    &name,
                    15.0,
                    Paint::Fg,
                    true,
                );
                if !more.is_empty() {
                    body.text(bx + 56.0, y + 37.0, &more, 13.0, Paint::Muted, false);
                }
                y += 44.0;
            }
            Widget::Card => {
                let detail = n.detail.as_deref().unwrap_or("");
                let lines = wrap(detail, 13.0, bw - 28.0);
                let h = 44.0 + lines.len() as f64 * 18.0;
                body.boxed(bx, y, bw, h, 10.0, Paint::Bg);
                body.text(bx + 14.0, y + 26.0, label, 15.0, Paint::Fg, true);
                for (k, l) in lines.iter().enumerate() {
                    body.text(
                        bx + 14.0,
                        y + 46.0 + k as f64 * 18.0,
                        l,
                        13.0,
                        Paint::Muted,
                        false,
                    );
                }
                y += h;
            }
            Widget::List => {
                let list = if its.is_empty() {
                    vec!["Item".to_string()]
                } else {
                    its.clone()
                };
                for (i, it) in list.iter().enumerate() {
                    let ry = y + i as f64 * 48.0;
                    body.circle(bx + 16.0, ry + 24.0, 14.0, Paint::Header);
                    body.text(bx + 42.0, ry + 29.0, it, 14.5, Paint::Fg, false);
                    body.text_at(
                        bx + bw - 4.0,
                        ry + 29.0,
                        "›",
                        18.0,
                        Paint::Muted,
                        false,
                        "end",
                    );
                    body.line(bx + 42.0, ry + 48.0, bx + bw, ry + 48.0, Paint::Grid, 1.0);
                }
                y += list.len() as f64 * 48.0;
            }
            Widget::Table => {
                let cols = if its.is_empty() {
                    vec!["Column".to_string()]
                } else {
                    its.clone()
                };
                let cwid = bw / cols.len() as f64;
                body.boxed(bx, y, bw, 4.0 * 36.0, 8.0, Paint::Bg);
                body.fill(bx + 1.0, y + 1.0, bw - 2.0, 35.0, Paint::Header, 7.0);
                for (k, c) in cols.iter().enumerate() {
                    body.text(
                        bx + 12.0 + k as f64 * cwid,
                        y + 23.0,
                        c,
                        13.0,
                        Paint::Fg,
                        true,
                    );
                    for r in 1..4 {
                        let ry = y + r as f64 * 36.0;
                        body.fill(
                            bx + 12.0 + k as f64 * cwid,
                            ry + 14.0,
                            (cwid - 24.0) * (0.5 + 0.1 * ((k + r) % 4) as f64),
                            8.0,
                            Paint::Grid,
                            4.0,
                        );
                    }
                }
                for r in 1..4 {
                    body.line(
                        bx,
                        y + r as f64 * 36.0,
                        bx + bw,
                        y + r as f64 * 36.0,
                        Paint::Grid,
                        1.0,
                    );
                }
                y += 4.0 * 36.0;
            }
            Widget::Tabs => {
                let k = its.len().max(1) as f64;
                let w = bw / k;
                for (i, it) in its.iter().enumerate() {
                    let on = i == 0;
                    body.text_at(
                        bx + w * (i as f64 + 0.5),
                        y + 20.0,
                        it,
                        14.0,
                        if on {
                            Paint::Color(accent(n))
                        } else {
                            Paint::Muted
                        },
                        on,
                        "middle",
                    );
                    if on {
                        body.fill(
                            bx + w * i as f64 + 8.0,
                            y + 32.0,
                            w - 16.0,
                            3.0,
                            Paint::Color(accent(n)),
                            1.5,
                        );
                    }
                }
                body.line(bx, y + 35.0, bx + bw, y + 35.0, Paint::Grid, 1.0);
                y += 36.0;
            }
            Widget::Breadcrumb => {
                let mut x = bx;
                for (i, it) in its.iter().enumerate() {
                    let last = i + 1 == its.len();
                    body.text(
                        x,
                        y + 15.0,
                        it,
                        13.0,
                        if last { Paint::Fg } else { Paint::Muted },
                        last,
                    );
                    x += text::width(it, 13.0) + 6.0;
                    if !last {
                        body.text(x, y + 15.0, "›", 13.0, Paint::Muted, false);
                        x += 14.0;
                    }
                }
                y += 20.0;
            }
            Widget::Badges => {
                let (mut x, mut row) = (bx, 0.0);
                for it in &its {
                    let w = text::width(it, 12.5) + 22.0;
                    if x + w > bx + bw && x > bx {
                        x = bx;
                        row += 32.0;
                    }
                    body.fill(x, y + row, w, 26.0, Paint::Header, 13.0);
                    body.text_at(
                        x + w / 2.0,
                        y + row + 17.5,
                        it,
                        12.5,
                        Paint::Fg,
                        false,
                        "middle",
                    );
                    x += w + 8.0;
                }
                y += row + 26.0;
            }
            Widget::Alert => {
                let lines = wrap(label, 13.5, bw - 44.0);
                let h = 20.0 + lines.len().max(1) as f64 * 19.0;
                let c = n.color.clone().unwrap_or_else(|| "yellow".into());
                body.fill(bx, y, bw, h, Paint::Soft(c.clone()), 8.0);
                body.fill(bx, y, 4.0, h, Paint::Color(c), 2.0);
                body.text(bx + 16.0, y + 24.0, "!", 15.0, Paint::Fg, true);
                for (k, l) in lines.iter().enumerate() {
                    body.text(
                        bx + 32.0,
                        y + 24.0 + k as f64 * 19.0,
                        l,
                        13.5,
                        Paint::Fg,
                        false,
                    );
                }
                y += h;
            }
            Widget::Progress => {
                if !label.is_empty() {
                    body.text(bx, y + 12.0, label, 12.5, Paint::Muted, true);
                    y += 20.0;
                }
                body.fill(bx, y, bw, 8.0, Paint::Header, 4.0);
                body.fill(bx, y, bw * 0.62, 8.0, Paint::Color(accent(n)), 4.0);
                y += 8.0;
            }
            Widget::Stepper => {
                let k = its.len().max(1) as f64;
                let w = bw / k;
                for (i, it) in its.iter().enumerate() {
                    let cx = bx + w * (i as f64 + 0.5);
                    if i + 1 < its.len() {
                        body.line(
                            cx + 14.0,
                            y + 14.0,
                            cx + w - 14.0,
                            y + 14.0,
                            Paint::Grid,
                            2.0,
                        );
                    }
                    let done = i == 0;
                    body.circle(
                        cx,
                        y + 14.0,
                        13.0,
                        if done {
                            Paint::Color(accent(n))
                        } else {
                            Paint::Grid
                        },
                    );
                    body.circle(
                        cx,
                        y + 14.0,
                        11.5,
                        if done {
                            Paint::Color(accent(n))
                        } else {
                            Paint::Bg
                        },
                    );
                    body.text_at(
                        cx,
                        y + 18.5,
                        &(i + 1).to_string(),
                        12.0,
                        if done { Paint::Bg } else { Paint::Muted },
                        true,
                        "middle",
                    );
                    body.text_at(
                        cx,
                        y + 44.0,
                        it,
                        12.0,
                        if done { Paint::Fg } else { Paint::Muted },
                        done,
                        "middle",
                    );
                }
                y += 50.0;
            }
            Widget::Pagination => {
                let pages = ["‹", "1", "2", "3", "…", "9", "›"];
                let w = 32.0;
                let x = bx + (bw - pages.len() as f64 * (w + 6.0)) / 2.0;
                for (i, p) in pages.iter().enumerate() {
                    let px = x + i as f64 * (w + 6.0);
                    if i == 1 {
                        body.fill(px, y, w, w, Paint::Color(accent(n)), 6.0);
                        body.text_at(px + w / 2.0, y + 21.0, p, 13.0, Paint::Bg, true, "middle");
                    } else {
                        body.boxed(px, y, w, w, 6.0, Paint::Bg);
                        body.text_at(px + w / 2.0, y + 21.0, p, 13.0, Paint::Fg, false, "middle");
                    }
                }
                y += w;
            }
            Widget::Rating => {
                body.text(
                    bx,
                    y + 20.0,
                    "★★★★☆",
                    20.0,
                    Paint::Color(n.color.clone().unwrap_or_else(|| "yellow".into())),
                    false,
                );
                if !label.is_empty() {
                    body.text(bx + 110.0, y + 19.0, label, 13.0, Paint::Muted, false);
                }
                y += 26.0;
            }
            Widget::Divider => {
                if label.is_empty() {
                    body.line(bx, y + 8.0, bx + bw, y + 8.0, Paint::Grid, 1.0);
                } else {
                    let w = text::width(label, 12.5) + 20.0;
                    body.line(bx, y + 8.0, bx + (bw - w) / 2.0, y + 8.0, Paint::Grid, 1.0);
                    body.text_at(
                        bx + bw / 2.0,
                        y + 12.5,
                        label,
                        12.5,
                        Paint::Muted,
                        false,
                        "middle",
                    );
                    body.line(
                        bx + (bw + w) / 2.0,
                        y + 8.0,
                        bx + bw,
                        y + 8.0,
                        Paint::Grid,
                        1.0,
                    );
                }
                y += 16.0;
            }
            Widget::Spacer => {
                y += 12.0;
            }
            Widget::Footer => {
                body.line(bx, y, bx + bw, y, Paint::Grid, 1.0);
                body.text_at(
                    bx + bw / 2.0,
                    y + 24.0,
                    label,
                    12.5,
                    Paint::Muted,
                    false,
                    "middle",
                );
                y += 32.0;
            }
            Widget::Sidebar => {
                // on a phone: a menu list
                for (i, it) in its.iter().enumerate() {
                    body.text(
                        bx,
                        y + 18.0 + i as f64 * 34.0,
                        it,
                        15.0,
                        if i == 0 { Paint::Fg } else { Paint::Muted },
                        i == 0,
                    );
                }
                y += its.len() as f64 * 34.0;
            }
        }
        rows.push((
            n.id.clone(),
            Rect {
                x: bx,
                y: start,
                w: bw,
                h: (y - start).max(8.0),
            },
        ));
        y += GAP;
    }
    // what sits at the bottom
    let tabbar = parts.iter().find(|n| n.widget == Some(Widget::Tabbar));
    let fab = parts.iter().find(|n| n.widget == Some(Widget::Fab));
    let bottom = if tabbar.is_some() { 64.0 } else { 0.0 };
    let h = (y + PAD + bottom + if fab.is_some() { 64.0 } else { 0.0 }).max(min_h);

    // the frame
    match device {
        Device::Web => {
            d.boxed(0.0, 0.0, fw, h, 10.0, Paint::Bg);
            d.fill(1.0, 1.0, fw - 2.0, chrome - 1.0, Paint::Header, 9.0);
            d.fill(1.0, chrome - 10.0, fw - 2.0, 10.0, Paint::Header, 0.0);
            for (k, c) in ["red", "yellow", "green"].iter().enumerate() {
                d.circle(
                    18.0 + k as f64 * 18.0,
                    chrome / 2.0,
                    5.5,
                    Paint::Soft(c.to_string()),
                );
            }
            d.fill(
                80.0,
                9.0,
                fw - 160.0,
                chrome - 18.0,
                Paint::Bg,
                (chrome - 18.0) / 2.0,
            );
            let url = format!("{}.app", crate::model::slug(g.display()));
            d.text_at(
                fw / 2.0,
                chrome / 2.0 + 4.5,
                &url,
                12.0,
                Paint::Muted,
                false,
                "middle",
            );
            d.line(1.0, chrome, fw - 1.0, chrome, Paint::Grid, 1.0);
        }
        _ => {
            // a phone or a tablet: a rounded body, the status bar
            d.fill(-8.0, -8.0, fw + 16.0, h + 16.0, Paint::Fg, 36.0);
            d.fill(0.0, 0.0, fw, h, Paint::Bg, 28.0);
            d.text(24.0, 26.0, "9:41", 13.0, Paint::Fg, true);
            d.text_at(fw - 24.0, 26.0, "▂▄▆ ◔", 11.0, Paint::Fg, false, "end");
            if device == Device::Mobile {
                d.fill(fw / 2.0 - 40.0, 10.0, 80.0, 22.0, Paint::Fg, 11.0);
                d.fill(fw / 2.0 - 60.0, h - 12.0, 120.0, 4.0, Paint::Fg, 2.0);
            }
        }
    }
    if let Some(s) = sidebar {
        let top = chrome;
        d.fill(1.0, top, side_w, h - top - 1.0, Paint::Header, 0.0);
        d.line(side_w, top, side_w, h - 1.0, Paint::Grid, 1.0);
        for (i, it) in items(&s.label).iter().enumerate() {
            let iy = top + 20.0 + i as f64 * 38.0;
            if i == 0 {
                d.fill(10.0, iy, side_w - 20.0, 32.0, Paint::Bg, 6.0);
            }
            d.text(
                24.0,
                iy + 21.0,
                it,
                14.0,
                if i == 0 { Paint::Fg } else { Paint::Muted },
                i == 0,
            );
        }
        rows.push((
            s.id.clone(),
            Rect {
                x: 0.0,
                y: top,
                w: side_w,
                h: h - top,
            },
        ));
    }
    d.marks.extend(body.marks);
    if let Some(t) = tabbar {
        let its = items(&t.label);
        let top = h - bottom - if device == Device::Mobile { 8.0 } else { 0.0 };
        d.line(0.0, top, fw, top, Paint::Grid, 1.0);
        let k = its.len().max(1) as f64;
        for (i, it) in its.iter().enumerate() {
            let cx = fw / k * (i as f64 + 0.5);
            let on = i == 0;
            let paint = if on {
                Paint::Color(t.color.clone().unwrap_or_else(|| "blue".into()))
            } else {
                Paint::Muted
            };
            d.fill(cx - 10.0, top + 12.0, 20.0, 20.0, paint.clone(), 6.0);
            d.text_at(cx, top + 48.0, it, 11.5, paint, on, "middle");
        }
        rows.push((
            t.id.clone(),
            Rect {
                x: 0.0,
                y: top,
                w: fw,
                h: bottom,
            },
        ));
    }
    if let Some(f) = fab {
        let (cx, cy) = (fw - 44.0, h - bottom - 52.0);
        d.circle(
            cx,
            cy,
            26.0,
            Paint::Color(f.color.clone().unwrap_or_else(|| "blue".into())),
        );
        d.text_at(cx, cy + 8.5, "+", 26.0, Paint::Bg, false, "middle");
        rows.push((
            f.id.clone(),
            Rect {
                x: cx - 26.0,
                y: cy - 26.0,
                w: 52.0,
                h: 52.0,
            },
        ));
    }
    Plot {
        w: fw,
        h,
        marks: d.marks,
        rows,
    }
}

/// Screen `gid`'s components as text, one per line (`button submit:
/// Sign in [blue]`), for editing.
pub fn text_of(doc: &Document, gid: &str) -> Option<String> {
    let g = doc.group(gid).filter(|g| g.screen.is_some())?;
    let mut out = String::new();
    crate::dsl::screen_block(doc, g, "", &mut out);
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

/// The ops that make screen `gid`'s components what `text` says: each line
/// read as in the file; a component keeps its id (and its arrows) when its
/// line keeps it, new ones are added, missing ones removed, all in order.
pub fn edit_ops(doc: &Document, gid: &str, text: &str) -> Result<Vec<serde_json::Value>, String> {
    use serde_json::json;
    let g = doc
        .group(gid)
        .filter(|g| g.screen.is_some())
        .ok_or_else(|| format!("no screen `{gid}`"))?;
    let body: String = text.lines().map(|l| format!("  {}\n", l.trim())).collect();
    let src = format!("screen {gid} {{\n{body}}}\n");
    let fresh = crate::dsl::parse(&src).map_err(|e| {
        e.iter()
            .map(|d| format!("line {}: {}", d.line.saturating_sub(1), d.message))
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let want: Vec<&Node> = fresh
        .nodes
        .iter()
        .filter(|n| n.group.as_deref() == Some(gid))
        .collect();
    let mut ops = Vec::new();
    for (i, n) in want.iter().enumerate() {
        let fields = json!({
            "label": n.label,
            "widget": n.widget.map(|w| w.as_str()).unwrap_or("text"),
            "color": n.color.clone().unwrap_or_default(),
            "detail": n.detail.clone().unwrap_or_default(),
            "link": n.link.clone().unwrap_or_default(),
            "index": i,
        });
        let mut op = if doc
            .node(&n.id)
            .is_some_and(|o| o.group.as_deref() == Some(g.id.as_str()))
        {
            json!({"op": "update_node", "id": n.id})
        } else if doc.id_taken(&n.id) {
            return Err(format!("id `{}` is already used", n.id));
        } else {
            json!({"op": "add_node", "id": n.id, "group": gid})
        };
        if let (Some(o), Some(f)) = (op.as_object_mut(), fields.as_object()) {
            for (k, v) in f {
                // a new one takes only what it has
                if o["op"] == "add_node" && v.as_str() == Some("") {
                    continue;
                }
                o.insert(k.clone(), v.clone());
            }
        }
        ops.push(op);
    }
    for n in doc.nodes.iter().filter(|n| n.group.as_deref() == Some(gid)) {
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

    const LOGIN: &str = "screen login: Sign in [mobile] {\n  header: Acme\n  title: Welcome back\n  input email: Email\n  password: Password\n  checkbox: Remember me\n  button submit: Sign in [blue]\n  link forgot: Forgot password?\n  divider: or\n  button google: Continue with Google\n}\nscreen home: Home [web] {\n  navbar: Acme | Docs | Pricing | Sign out\n  sidebar: Inbox | Sent | Drafts\n  breadcrumb: Mail | Inbox\n  search: Search mail\n  tabs: All | Unread | Starred\n  list: Ana | Ben | Cleo\n  tabbar: Home | Search | Me\n  fab: New\n}\n\nlogin.submit -> home: signs in\n";

    #[test]
    fn a_screen_round_trips_and_lays_out_its_components() {
        let doc = dsl::parse(LOGIN).unwrap();
        let g = doc.group("login").unwrap();
        assert_eq!(g.screen, Some(Device::Mobile));
        let ids: Vec<&str> = doc
            .nodes
            .iter()
            .filter(|n| n.group.as_deref() == Some("login"))
            .map(|n| n.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "login.header",
                "login.title",
                "login.email",
                "login.password",
                "login.checkbox",
                "login.submit",
                "login.forgot",
                "login.divider",
                "login.google"
            ]
        );
        assert_eq!(
            doc.node("login.submit").unwrap().widget,
            Some(Widget::Button)
        );
        assert!(
            doc.edge("login.submit->home").is_some(),
            "a flow from a button"
        );
        assert_eq!(dsl::print(&doc), LOGIN);
        assert_eq!(crate::from_json(&crate::to_json(&doc)).unwrap(), doc);
        // components top to bottom, inside the phone
        let p = plot(&doc, g);
        assert_eq!(p.rows.len(), 9);
        for w in p.rows.windows(2) {
            assert!(
                w[0].1.y + w[0].1.h <= w[1].1.y + 0.5,
                "{:?} then {:?}",
                w[0],
                w[1]
            );
        }
        assert!(p
            .rows
            .iter()
            .all(|(_, r)| r.x >= 0.0 && r.x + r.w <= p.w + 0.5));
        // a web page: the sidebar on the left, the tab bar at the bottom
        let home = plot(&doc, doc.group("home").unwrap());
        let at = |id: &str| home.rows.iter().find(|r| r.0 == id).unwrap().1;
        assert!(at("home.sidebar").x < at("home.list").x);
        assert!(at("home.tabbar").y > at("home.list").y);
        assert_eq!(home.w, 1024.0);
        // ops: a screen and its components, written back as a block
        let mut s = crate::Session::new();
        s.set_source("title: App\n");
        let r = s.apply(r#"{"ops":[{"op":"add_group","id":"signup","label":"Sign up","screen":"web"},{"op":"add_node","id":"go","label":"Create account","group":"signup","widget":"button","color":"blue"},{"op":"add_node","label":"Name","group":"signup","widget":"input"}]}"#);
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("screen signup: Sign up [web] {\n  button go: Create account [blue]\n  input name: Name\n}"),
            "{out}"
        );
        assert_eq!(
            s.apply(r#"{"op":"add_node","label":"x","group":"signup","widget":"blink"}"#)["ok"],
            false
        );
        // renamed, a component keeps the screen's prefix
        assert_eq!(
            s.apply(r#"{"op":"rename","id":"signup.go","to":"create"}"#)["ok"],
            true
        );
        assert!(s.doc.node("signup.create").is_some());
        assert!(s
            .source("dsl")
            .unwrap()
            .contains("  button create: Create account [blue]\n"));
        // edited as text: kept ids keep their arrows; new, gone and moved ones
        let mut s = crate::Session::new();
        s.set_source(LOGIN);
        let t = text_of(&s.doc, "login").unwrap();
        assert!(t.starts_with("header: Acme\ntitle: Welcome back\n"), "{t}");
        let ops = edit_ops(&s.doc, "login", "title: Hello again\nbutton submit: Log in [green]\ninput email: E-mail\ntoggle: Stay signed in").unwrap();
        let r = s.apply(&serde_json::json!({ "ops": ops }).to_string());
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("screen login: Sign in [mobile] {\n  title: Hello again\n  button submit: Log in [green]\n  input email: E-mail\n  toggle: Stay signed in\n}"),
            "{out}"
        );
        assert!(out.contains("login.submit -> home: signs in"), "{out}");
        assert!(edit_ops(&s.doc, "login", "blink: x").is_err());
        // unknown components are refused with the list
        let e = dsl::parse("screen s {\n  blink: Hi\n}\n").unwrap_err();
        assert!(format!("{e:?}").contains("not a screen component"), "{e:?}");
    }
}
