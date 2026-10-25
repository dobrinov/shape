//! shape-core: lightweight, text-first diagrams.
//!
//! ```
//! let doc = shape_core::parse("api -> db: SQL\ncomment on api: cache this?").unwrap();
//! let svg = shape_core::render_svg(&doc, &Default::default());
//! let prompt = shape_core::prompt::prompt(&doc, &Default::default());
//! assert!(svg.starts_with("<svg") && prompt.contains("cache this?"));
//! ```
//!
//! - [`model`]: the document (also the JSON format)
//! - [`dsl`]: the human-friendly text format
//! - [`mermaid`]: Mermaid flowchart import/export
//! - [`mindmap`]: mindmaps (outlines, tree layout, Markdown and AI prompts)
//! - [`ops`]: edit operations (the API for editors, tools and LLMs)
//! - [`layout`], [`route`], [`render`]: placement, edges and SVG
//! - [`diff`]: comparing two versions
//! - [`prompt`]: briefs and review prompts for LLMs
//! - [`Session`]: a stateful wrapper used by the WASM build and editors
// the JSON Schema is one large `json!` literal
#![recursion_limit = "256"]
pub mod adr;
pub mod charts;
pub mod cycle;
pub mod dashboard;
pub mod diff;
pub mod dsl;
pub mod entities;
pub mod forecast;
pub mod gantt;
pub mod layout;
pub mod merge;
pub mod mermaid;
pub mod mindmap;
pub mod model;
pub mod ops;
pub mod patch;
pub mod perspectives;
pub mod prompt;
pub mod qa;
pub mod query;
pub mod recipes;
pub mod render;
pub mod risks;
pub mod route;
pub mod screen;
pub mod sequence;
pub mod tables;
pub mod text;
pub mod theme;
pub mod timeline;
pub mod schemas;
pub mod todo;
pub mod upgrades;

use serde::Serialize;
use serde_json::{json, Value};

pub use model::{Diagnostic, Document};
pub use ops::Op;
pub use render::{RenderOptions, Rendered};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Dsl,
    Json,
    Mermaid,
}

impl Format {
    pub fn parse(s: &str) -> Option<Format> {
        match s.to_ascii_lowercase().as_str() {
            "dsl" | "shape" | "diagram" | "text" => Some(Format::Dsl),
            "json" => Some(Format::Json),
            "mermaid" | "mmd" => Some(Format::Mermaid),
            _ => None,
        }
    }
}

/// Guess the format of a source text.
pub fn detect(src: &str) -> Format {
    let t = src.trim_start();
    if t.starts_with('{') {
        Format::Json
    } else if mermaid::looks_like(src) {
        Format::Mermaid
    } else {
        Format::Dsl
    }
}

/// Parse DSL, JSON or Mermaid (detected).
pub fn parse(src: &str) -> Result<Document, Vec<Diagnostic>> {
    parse_as(src, detect(src))
}

pub fn parse_as(src: &str, format: Format) -> Result<Document, Vec<Diagnostic>> {
    match format {
        Format::Dsl => dsl::parse(src),
        Format::Json => from_json(src),
        Format::Mermaid => mermaid::import(src),
    }
}

pub fn from_json(src: &str) -> Result<Document, Vec<Diagnostic>> {
    let mut doc: Document = serde_json::from_str(src).map_err(|e| {
        vec![Diagnostic::new(
            e.line(),
            format!("invalid diagram JSON: {e}"),
        )]
    })?;
    doc.normalize();
    let problems = doc.check();
    if problems.is_empty() {
        Ok(doc)
    } else {
        Err(problems)
    }
}

pub fn to_json(doc: &Document) -> String {
    serde_json::to_string_pretty(doc).unwrap()
}

/// Export as `dsl`, `json`, `mermaid`, `brief` (Markdown for LLMs) or `svg`.
pub fn export(doc: &Document, format: &str) -> Result<String, String> {
    Ok(match format {
        "dsl" | "shape" | "diagram" => dsl::print(doc),
        "json" => to_json(doc) + "\n",
        "mermaid" | "mmd" => mermaid::export(doc),
        "brief" | "md" | "markdown" | "llm" => prompt::brief(doc),
        "svg" => render_svg(doc, &RenderOptions::default()),
        f => {
            return Err(format!(
                "unknown format `{f}` (dsl, json, mermaid, brief, svg)"
            ))
        }
    })
}

pub fn render_svg(doc: &Document, opts: &RenderOptions) -> String {
    render::render(doc, opts).svg
}

/// Render `new` with what changed since `old` highlighted.
/// How many elements changed from `old` to `new` (added, removed or
/// changed), canvases included.
pub fn change_count(old: &Document, new: &Document) -> usize {
    fn walk(old: &Document, new: &Document) -> usize {
        let d = diff::diff(old, new);
        let here = d.entries.iter().filter(|e| e.kind != "setting").count();
        let empty = Document::default();
        here + new
            .canvases
            .iter()
            .map(|c| walk(old.canvas(&c.id).map(|o| &o.doc).unwrap_or(&empty), &c.doc))
            .sum::<usize>()
    }
    walk(old, new)
}

pub fn render_diff(old: &Document, new: &Document, opts: &RenderOptions) -> Rendered {
    let (merged, changes) = diff::merge(old, new);
    let mut o = opts.clone();
    o.changes = changes;
    render::render(&merged, &o)
}

/// The language reference (Markdown), for people and for LLM context.
pub fn spec() -> String {
    include_str!("../LANGUAGE.md").replace("OPS_REFERENCE", prompt::OPS_REFERENCE)
}

/// JSON Schema of the document format.
pub fn json_schema() -> Value {
    let e = |v: &[&str]| json!({"type": "string", "enum": v});
    let names = |all: Vec<&'static str>| all;
    let shapes = names(model::Shape::ALL.iter().map(|s| s.as_str()).collect());
    let color = json!({"type": "string", "description": "gray, blue, green, yellow, orange, red, purple, pink, teal or #rrggbb"});
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/dobrinov/shape/schema.json",
        "title": "Shape board",
        "type": "object",
        "required": ["nodes"],
        "additionalProperties": false,
        "properties": {
            "title": {"type": "string"},
            "description": {"type": "string"},
            "settings": {"type": "object", "additionalProperties": false, "properties": {
                "direction": e(&["right", "down", "left", "up"]),
                "layout": e(&["layered", "grid"]),
                "routing": e(&["orthogonal", "curved", "straight"]),
                "theme": e(&["light", "dark"])
            }},
            "nodes": {"type": "array", "items": {"type": "object", "required": ["id"], "additionalProperties": false, "properties": {
                "id": {"type": "string", "pattern": "^[\\p{L}\\p{N}_]([\\p{L}\\p{N}_]|[-.](?=[\\p{L}\\p{N}_]))*$"},
                "label": {"type": "string", "description": "defaults to the id; \\n breaks lines"},
                "shape": e(&shapes),
                "detail": {"type": "string", "description": "smaller second line"},
                "color": color,
                "group": {"type": "string", "description": "id of the enclosing group"},
                "link": {"type": "string", "description": "a URL, or #id to go to another element on the board"},
                "image": {"type": "string", "description": "the id of an asset (an image) the box shows"},
                "canvas": {"type": "string", "description": "the id of a canvas inside the board that the box opens"},
                "tags": {"type": "array", "items": {"type": "string"}},
                "x": {"type": "number"}, "y": {"type": "number"}, "w": {"type": "number"}, "h": {"type": "number"},
                "parent": {"type": "string", "description": "mindmap: id of the topic this one branches from"},
                "mindmap": e(&["right", "left", "both", "down"]),
                "collapsed": {"type": "boolean", "description": "mindmap: branches under this topic are folded"},
                "start": {"type": "string", "format": "date", "description": "gantt task: start date (YYYY-MM-DD)"},
                "days": {"type": "number", "exclusiveMinimum": 0, "description": "gantt task: duration in days"},
                "after": {"type": "array", "items": {"type": "string"}, "description": "gantt task: ids of the tasks it starts after (in any chart)"},
                "milestone": {"type": "boolean", "description": "gantt task: a point in time (drawn as a diamond), not a bar"},
                "done": {"type": "number", "minimum": 0, "maximum": 100, "description": "gantt task: percent done"},
                "owner": {"type": "string", "description": "gantt task or risk: who does it (owns it)"},
                "probability": {"type": "integer", "minimum": 1, "maximum": 5, "description": "risk: how likely"},
                "impact": {"type": "integer", "minimum": 1, "maximum": 5, "description": "risk: how bad"},
                "values": {"type": "array", "items": {"type": ["number", "null"]}, "description": "chart row: one value per series"},
                "cells": {"type": "array", "items": {"type": "string"}, "description": "table row: its cells after the first (the label)"},
                "date": {"type": "string", "description": "timeline event: when, as written (2026-10-28, 28 Oct 2026, Oct 2026, Q4 2026, 2026, or a span: 7–14 May 2026)"},
                "important": {"type": "boolean", "description": "timeline event: one that matters most (drawn as a star)"},
                "column": {"type": "object", "description": "database table column", "additionalProperties": false, "properties": {
                    "type": {"type": "string"}, "pk": {"type": "boolean"}, "unique": {"type": "boolean"}, "null": {"type": "boolean"},
                    "fk": {"type": "string", "description": "the referenced column's id (teams.id) or a table (its primary key)"}, "default": {"type": "string"}, "rel": {"type": "string", "description": "the relationship's name, written on the foreign key's line"}
                }}
            }}},
            "edges": {"type": "array", "items": {"type": "object", "required": ["from", "to"], "additionalProperties": false, "properties": {
                "id": {"type": "string", "description": "defaults to from->to"},
                "from": {"type": "string"}, "to": {"type": "string"},
                "label": {"type": "string"},
                "arrow": e(&["end", "start", "both", "none"]),
                "line": e(&["solid", "dashed", "dotted", "thick"]),
                "color": color
            }}},
            "groups": {"type": "array", "items": {"type": "object", "required": ["id"], "additionalProperties": false, "properties": {
                "id": {"type": "string"}, "label": {"type": "string"}, "color": color, "parent": {"type": "string"},
                "lanes": {"type": "boolean", "description": "swimlanes: the groups inside are lanes"},
                "adr": {"type": "string", "enum": ["proposed", "accepted", "rejected", "superseded", "deprecated"], "description": "a decision record with this status: its nodes are its fields (label = field name, detail = text)"},
                "sequence": {"type": "boolean", "description": "a sequence diagram: its nodes are participants side by side; arrows between them are messages, top to bottom in document order"},
                "kanban": {"type": "boolean", "description": "a kanban board (with lanes): its lanes are columns of cards, stacked in document order"},
                "gantt": {"type": "boolean", "description": "gantt chart: the nodes inside are tasks"},
                "start": {"type": "string", "format": "date", "description": "gantt chart: its first day (YYYY-MM-DD)"},
                "scale": e(&["day", "week", "month", "quarter"]),
                "risks": {"type": "boolean", "description": "risk matrix: the nodes inside are risks"},
                "levels": {"type": "integer", "minimum": 3, "maximum": 5, "description": "risk matrix: levels of probability and impact (5 by default)"},
                "chart": e(&["bar", "line", "area", "pie", "donut", "scatter"]),
                "stacked": {"type": "boolean", "description": "chart: series stacked"},
                "horizontal": {"type": "boolean", "description": "bar chart: bars across"},
                "unit": {"type": "string", "description": "chart: what values are in ($ € £ ¥ go in front)"},
                "series": {"type": "array", "items": {"type": "string"}, "description": "chart: the series' names"},
                "axis": {"type": "string", "description": "chart: what the rows are"},
                "table": {"type": "boolean", "description": "table: the nodes inside are rows"},
                "timeline": {"type": "boolean", "description": "timeline: the nodes inside are events, each with a date"},
                "cycle": {"type": "boolean", "description": "cycle: the nodes inside are steps on a circle, each leading to the next and the last back to the first"},
                "entity": {"type": "boolean", "description": "database table: the nodes inside are columns"},
                "columns": {"type": "array", "items": {"type": "string"}, "description": "table: the columns' names"},
                "w": {"type": "number"}, "h": {"type": "number"},
                "x": {"type": "number", "description": "swimlanes / gantt chart: pinned left edge"},
                "y": {"type": "number", "description": "swimlanes / gantt chart: pinned top edge"}
            }}},
            "canvases": {"type": "array", "description": "boards inside this one: each has an id and every field of a document (its title is its name)", "items": {"type": "object", "required": ["id"]}},
            "assets": {"type": "array", "items": {"type": "object", "required": ["id", "data"], "additionalProperties": false, "properties": {
                "id": {"type": "string"},
                "data": {"type": "string", "description": "a data:image/png;base64,… (or jpeg, gif, webp, svg+xml) URI"}
            }}},
            "comments": {"type": "array", "items": {"type": "object", "required": ["text"], "additionalProperties": false, "properties": {
                "id": {"type": "string", "description": "defaults to c1, c2, …"},
                "on": {"type": "string", "description": "id of a node, edge or group; omit for a canvas comment"},
                "text": {"type": "string"},
                "author": {"type": "string"},
                "status": e(&["open", "resolved"]),
                "resolution": {"type": "string"},
                "replies": {"type": "array", "items": {"type": "object", "required": ["text"], "additionalProperties": false, "properties": {"author": {"type": "string"}, "text": {"type": "string"}}}},
                "x": {"type": "number"}, "y": {"type": "number"}
            }}}
        }
    })
}

/// A document being edited: the current version, the last parse errors and
/// an optional base version to diff against. Every method takes and returns
/// JSON-friendly values so it maps 1:1 onto the WASM API.
#[derive(Debug, Clone, Default)]
pub struct Session {
    pub doc: Document,
    pub base: Option<Document>,
    pub errors: Vec<Diagnostic>,
    /// The DSL as it was loaded, and what it said: `source("dsl")` keeps it
    /// as written and rewrites only what changed since (see [`patch`]).
    written: Option<(String, Document)>,
    /// Themes from files the board names (`theme: ./brand.theme`).
    pub themes: Vec<model::ThemeDef>,
    /// With a canvas open: the whole board (its copy of the canvas is stale
    /// while `doc` is edited) and the canvas's path from the top.
    outer: Option<(Document, Vec<String>)>,
    /// A team's own recipes (`recipes/*.shape`), for prompts with the
    /// built-in ones (see [`recipes`]).
    pub recipes: Vec<recipes::Recipe>,
}

impl Session {
    pub fn new() -> Session {
        Session::default()
    }

    /// Load source text (format detected). On errors in DSL input, the valid
    /// statements are still loaded so editors keep showing something useful.
    pub fn set_source(&mut self, src: &str) -> Value {
        let format = detect(src);
        let path = self.canvas_path();
        let (doc, errors) = match format {
            Format::Dsl => dsl::parse_lenient(src),
            _ => match parse_as(src, format) {
                Ok(d) => (d, Vec::new()),
                Err(e) => (self.whole(), e),
            },
        };
        self.written = (format == Format::Dsl).then(|| (src.to_string(), doc.clone()));
        self.doc = doc;
        self.outer = None;
        self.errors = errors;
        // the canvas that was open stays open, while it is there
        if !path.is_empty() {
            let _ = self.open_canvas(&path);
        }
        json!({"ok": self.errors.is_empty(), "format": format, "errors": self.errors})
    }

    pub fn set_document(&mut self, doc: Document) {
        self.doc = doc;
        self.outer = None;
        self.errors.clear();
        self.written = None;
    }

    /// The whole board, with what was done in the open canvas in place.
    pub fn whole(&self) -> Document {
        match &self.outer {
            None => self.doc.clone(),
            Some((root, path)) => {
                let mut r = root.clone();
                if let Some(d) = r.at_mut(path) {
                    *d = self.doc.clone();
                }
                r
            }
        }
    }

    /// The open canvas: its ids from the top board (empty: the top board).
    pub fn canvas_path(&self) -> Vec<String> {
        self.outer.as_ref().map(|o| o.1.clone()).unwrap_or_default()
    }

    /// Open the canvas at `path` (ids from the top board; empty: the top).
    /// From then on `doc` is that canvas: ops, prompts and drawing are about
    /// it, and `source` is still the whole board.
    pub fn open_canvas(&mut self, path: &[String]) -> Result<(), String> {
        let root = self.whole();
        let d = root
            .at(path)
            .ok_or_else(|| format!("no canvas `{}`", path.join("/")))?
            .clone();
        self.doc = d;
        self.outer = (!path.is_empty()).then(|| (root, path.to_vec()));
        Ok(())
    }

    /// What a summary is of, as of day `today`: the project whose canvas is
    /// open (on a board of several projects, each has a summary of its own,
    /// and so on down), else the whole board. The summary's rows have paths
    /// from there.
    pub fn summary_path(&self, today: i64) -> Vec<String> {
        let root = self.whole();
        let mut cur = &root;
        let mut out = Vec::new();
        for id in self.canvas_path() {
            let ms = dashboard::milestones(cur, today);
            if !dashboard::parts(cur, &ms).iter().any(|(c, _)| *c == id) {
                break;
            }
            let Some(c) = cur.canvas(&id) else { break };
            cur = &c.doc;
            out.push(id);
        }
        out
    }

    /// The titles along the open canvas's path, the top board first.
    pub fn canvas_trail(&self) -> Vec<(Vec<String>, String)> {
        let root = self.whole();
        let path = self.canvas_path();
        let mut out = vec![(
            Vec::new(),
            root.title.clone().unwrap_or_else(|| "Board".into()),
        )];
        let mut cur = &root;
        for (k, id) in path.iter().enumerate() {
            let Some(c) = cur.canvas(id) else { break };
            out.push((path[..=k].to_vec(), c.display().to_string()));
            cur = &c.doc;
        }
        out
    }

    pub fn source(&self, format: &str) -> Result<String, String> {
        let whole = self.whole();
        match (&self.written, format) {
            (Some((src, old)), "dsl" | "shape" | "diagram") => Ok(patch::patch(src, old, &whole)),
            _ => export(&whole, format),
        }
    }

    /// Replace the open canvas (the whole board when none is open) with
    /// `doc`; the source keeps everything else as written.
    pub fn replace_here(&mut self, doc: Document) {
        self.doc = doc;
    }

    /// The base version of the open canvas.
    fn base_here(&self) -> Option<Document> {
        let b = self.base.as_ref()?;
        b.at(&self.canvas_path()).cloned()
    }

    pub fn set_base(&mut self, src: Option<&str>) -> Value {
        match src {
            None => {
                self.base = None;
                json!({"ok": true})
            }
            Some(s) => match parse(s) {
                Ok(d) => {
                    self.base = Some(d);
                    json!({"ok": true})
                }
                Err(e) => json!({"ok": false, "errors": e}),
            },
        }
    }

    /// Apply ops (JSON or an LLM reply containing JSON).
    pub fn apply(&mut self, ops_text: &str) -> Value {
        let ops = match ops::parse_ops(ops_text) {
            Ok(o) => o,
            Err(e) => return json!({"ok": false, "error": e}),
        };
        self.apply_ops(&ops)
    }

    /// Apply ops to the top board, also while a canvas is open (for what
    /// belongs to the whole board: its type, its perspectives).
    pub fn apply_top(&mut self, ops_text: &str) -> Value {
        let ops = match ops::parse_ops(ops_text) {
            Ok(o) => o,
            Err(e) => return json!({"ok": false, "error": e}),
        };
        let path = self.canvas_path();
        if path.is_empty() {
            return self.apply_ops(&ops);
        }
        let mut root = self.whole();
        match ops::apply(&mut root, &ops) {
            Ok(r) => {
                self.doc = root.at(&path).cloned().unwrap_or_default();
                self.outer = Some((root, path));
                json!({"ok": true, "created": r.created, "count": r.count})
            }
            Err(e) => json!({"ok": false, "error": e.to_string(), "index": e.index}),
        }
    }

    pub fn apply_ops(&mut self, ops: &[Op]) -> Value {
        match ops::apply(&mut self.doc, ops) {
            Ok(r) => json!({"ok": true, "created": r.created, "count": r.count}),
            Err(e) => json!({"ok": false, "error": e.to_string(), "index": e.index}),
        }
    }

    pub fn render(&self, opts: &RenderOptions) -> Rendered {
        let mut o = opts.clone();
        o.themes.extend(self.themes.iter().cloned());
        // a perspective's dashboard: computed from the whole board, or in a
        // project's canvas, from that project (see `summary_path`)
        if let (Some(p), true) = (&o.perspective, o.dashboard) {
            let today = o
                .today
                .as_deref()
                .and_then(gantt::parse_date)
                .unwrap_or_else(|| gantt::parse_date("2026-01-01").unwrap_or(0));
            let whole = self.whole();
            let path = self.summary_path(today);
            let before = o
                .before
                .as_deref()
                .and_then(|s| parse(s).ok())
                .and_then(|b| b.at(&path).cloned());
            let doc = whole.at(&path).unwrap_or(&whole);
            let mut doc = dashboard::build(doc, p, today, before.as_ref());
            // what the viewer unfolded (a project on the roadmap, its parts)
            for id in o.open.iter().flatten() {
                if let Some(n) = doc.node_mut(id) {
                    n.collapsed = false;
                }
            }
            return render::render(&doc, &o);
        }
        // a perspective shows its part of the board (and of the base, to compare)
        // starting high level: folded but for what is opened
        let seen = |d: &Document| match (&o.perspective, &o.open) {
            (Some(p), Some(open)) => perspectives::fold(&perspectives::view(d, p), p, open),
            (Some(p), None) => perspectives::view(d, p),
            _ => d.clone(),
        };
        let doc = seen(&self.doc);
        match self.base_here() {
            Some(b) => render_diff(&seen(&b), &doc, &o),
            None => render::render(&doc, &o),
        }
    }

    /// The board's perspectives: the built-in ones of its type, then its own.
    pub fn perspectives(&self) -> Vec<model::Perspective> {
        perspectives::all(&self.whole())
    }

    /// A theme file the board names (`theme: ./brand.theme`), by that name.
    pub fn add_theme_file(&mut self, name: &str, src: &str) -> Result<(), String> {
        let t = theme::from_file(name, src)?;
        self.themes.retain(|x| x.id != t.id);
        self.themes.push(t);
        Ok(())
    }

    pub fn diff_markdown(&self) -> String {
        match self.base_here() {
            Some(b) => diff::markdown(&diff::diff(&b, &self.doc), &b, &self.doc),
            None => "No base version to compare with.\n".into(),
        }
    }

    pub fn prompt(&self, opts: &prompt::PromptOptions) -> String {
        prompt::prompt(&self.doc, opts)
    }

    /// A prompt asking `question` about the whole board, canvases included,
    /// after the `history` of this conversation (see [`prompt::question`]).
    pub fn question_prompt(
        &self,
        history: &[(String, String)],
        question: &str,
        today: Option<i64>,
    ) -> String {
        prompt::question(&self.whole(), history, question, today)
    }

    /// A prompt asking `question` about the whole board, answered with a
    /// small board of the model's own (see [`prompt::answer_board`]).
    pub fn answer_board_prompt(
        &self,
        history: &[(String, String)],
        question: &str,
        today: Option<i64>,
    ) -> String {
        prompt::answer_board(&self.whole(), history, question, today, &self.recipes)
    }

    /// The places on the board an answer points at (see [`prompt::answer_refs`]).
    pub fn answer_refs(&self, answer: &str) -> Vec<prompt::AnswerRef> {
        prompt::answer_refs(&self.whole(), answer)
    }

    /// A prompt to rethink the whole board, from the open canvas (see
    /// [`prompt::rethink`]). With `since` (an earlier version of the whole
    /// board, any format; "": none of it existed), what changed since is new
    /// information; `label` says what that version is. Its ops apply to the
    /// top board (`in` ops reach into canvases).
    pub fn rethink_prompt(
        &self,
        since: Option<&str>,
        label: &str,
        instructions: Option<&str>,
    ) -> Result<String, String> {
        let old = match since {
            None => None,
            Some(s) if s.trim().is_empty() => Some(Document::default()),
            Some(s) => Some(parse(s).map_err(|e| {
                let e: Vec<String> = e.iter().map(|d| d.to_string()).collect();
                format!("the earlier version: {}", e.join("; "))
            })?),
        };
        Ok(prompt::rethink(
            &self.whole(),
            &self.canvas_path(),
            old.as_ref().map(|o| (o, label)),
            instructions,
            &self.recipes,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_convert_losslessly() {
        let src = "title: T\ngroup g: G {\n a: A [cylinder, blue]\n}\na -> b: x [dashed]\ncomment on a by me: hm\nreply c1: ok";
        let doc = parse(src).unwrap();
        let json = to_json(&doc);
        assert_eq!(detect(&json), Format::Json);
        let back = parse(&json).unwrap();
        assert_eq!(back, doc);
        let mmd = export(&doc, "mermaid").unwrap();
        assert_eq!(detect(&mmd), Format::Mermaid);
        assert!(spec().contains("add_node") && !spec().contains("OPS_REFERENCE"));
        assert!(json_schema()["properties"]["nodes"].is_object());
    }

    #[test]
    fn answers_are_small_boards_linked_back() {
        let mut s = Session::new();
        s.set_source("title: Checkout\npay: Payments [canvas: plan]\n\ncanvas plan: Plan {\n  sso: SSO login\n}\n");
        let p = s.answer_board_prompt(&[], "What blocks SSO?", None);
        assert!(
            p.contains("## Canvas `plan`")
                && p.contains("[link: #/plan/id]")
                && p.contains("## The question\n\nWhat blocks SSO?"),
            "{p}"
        );
        let (summary, source) = prompt::answer_parts("SSO waits on the IdP.\n\n```shape\ntitle: SSO\nsso: SSO login [link: #/plan/sso]\nidp: Pick an IdP [sticky]\nidp -> sso\n```\nThat is all.").unwrap();
        assert_eq!(summary, "SSO waits on the IdP.\nThat is all.");
        let d = dsl::parse(&source).unwrap();
        assert_eq!(d.node("sso").unwrap().link.as_deref(), Some("#/plan/sso"));
        // most answers are words only: no drawing
        assert_eq!(
            prompt::answer_parts("It waits on [[plan/sso]].\n- one\n").unwrap(),
            ("It waits on [[plan/sso]].\n- one".to_string(), String::new())
        );
        assert!(prompt::answer_parts("  ").is_err());
        assert!(prompt::answer_parts("```shape\na -> -> [\n```").is_err());
        // the fence's name from before the rename still works
        assert!(prompt::answer_parts("Here.\n\n```diagram\na: A\n```").is_ok());
    }

    #[test]
    fn questions_read_every_canvas_and_answers_point_into_them() {
        let src = "title: Checkout\npay: Payments [canvas: plan]\napi: Orders API\n\ncanvas plan: Delivery plan {\n  sso: SSO login [owner: ana]\n  canvas deep: Details {\n    okta: Use Okta\n  }\n}\n";
        let mut s = Session::new();
        s.set_source(src);
        s.open_canvas(&["plan".into()]).unwrap();
        let hist = vec![("Who owns SSO?".to_string(), "ana [[plan/sso]]".to_string())];
        let p = s.question_prompt(&hist, "And what was decided?", None);
        assert!(
            p.contains("## The top board") && p.contains("Orders API"),
            "{p}"
        );
        assert!(
            p.contains("## Canvas `plan` (“Delivery plan”)") && p.contains("SSO login"),
            "{p}"
        );
        assert!(
            p.contains("## Canvas `plan/deep` (“Details”)") && p.contains("Use Okta"),
            "{p}"
        );
        assert!(
            p.contains("Q: Who owns SSO?\nA: ana [[plan/sso]]") && p.ends_with("code blocks.\n")
        );
        let refs = s.answer_refs("See [[api]], [[plan/sso]] and [[plan/deep/okta]], all of [[plan]]; [[nope]]; [[api]] again");
        let got: Vec<(Vec<String>, Option<&str>, Option<&str>)> = refs
            .iter()
            .map(|r| (r.path.clone(), r.id.as_deref(), r.label.as_deref()))
            .collect();
        let p = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            got,
            vec![
                (p(&[]), Some("api"), Some("Orders API")),
                (p(&["plan"]), Some("sso"), Some("SSO login")),
                (p(&["plan", "deep"]), Some("okta"), Some("Use Okta")),
                (p(&["plan"]), None, Some("Delivery plan")),
                (p(&[]), Some("nope"), None),
            ]
        );
        // written loosely: the element with that id, wherever it is
        let loose = s.answer_refs("[[sso]] and [[deep/okta]]");
        assert_eq!((loose[0].path.clone(), loose[0].label.as_deref()), (p(&["plan"]), Some("SSO login")));
        assert_eq!((loose[1].path.clone(), loose[1].label.as_deref()), (p(&["plan", "deep"]), Some("Use Okta")));
        // a comment: its thread
        let mut c = Session::new();
        c.set_source("a: A\n\ncanvas plan: Plan {\n  b: B\n  comment on b: Which date is right for the launch, 26 Nov or 3 Dec?\n}\n");
        let r = &c.answer_refs("as [[c1]] asks")[0];
        assert!(r.comment && r.path == p(&["plan"]));
        assert_eq!(r.label.as_deref(), Some("comment: Which date is right for the…"));
    }

    #[test]
    fn rethink_reads_every_canvas_with_open_comments_first() {
        let old = "title: Checkout\npay: Payments [canvas: payments]\n\ncanvas payments: Payments flow {\n  intent: Create intent\n}\n";
        let now = "title: Checkout\npay: Payments [canvas: payments]\nlate: Top-level note\n\ncanvas payments: Payments flow {\n  intent: Create intent\n  sca: Banks ask for 3DS [sticky]\n  comment on intent by ana: We ship two card products now\n  reply c1 by bo: and price per card\n}\n";
        let mut s = Session::new();
        s.set_source(now);
        s.open_canvas(&["payments".into()]).unwrap();
        let p = s
            .rethink_prompt(Some(old), "where this branch left main", None)
            .unwrap();
        assert!(p.starts_with("# Rethink the board \"Checkout\""), "{p}");
        assert!(
            p.contains("looking at canvas `payments`")
                && p.contains("## The top board")
                && p.contains("## Canvas `payments` (“Payments flow”)"),
            "{p}"
        );
        // the comment, where it is and its reply
        assert!(p.contains("- `c1` in canvas `payments`, on node `intent` (\"Create intent\") by ana: We ship two card products now\n  - reply, bo: and price per card"), "{p}");
        // what is new, per canvas
        let new = &p[p.find("## What is new").unwrap()..p.find("## What to do").unwrap()];
        assert!(
            new.contains("### The top board")
                && new.contains("Top-level note")
                && new.contains("### Canvas `payments`")
                && new.contains("Banks ask for 3DS")
                && !new.contains("Create intent"),
            "{new}"
        );
        assert!(p.contains(r#"{"op": "in", "canvas": "plan""#));
        assert!(!s
            .rethink_prompt(None, "", None)
            .unwrap()
            .contains("What is new"));
        assert!(!s
            .rethink_prompt(Some(now), "the last commit", None)
            .unwrap()
            .contains("What is new"));
        assert!(s.rethink_prompt(Some("a -> -> ["), "x", None).is_err());
        // the person's own instructions replace the default; the rules stay
        let own = s
            .rethink_prompt(None, "", Some("Adapt this canvas for senior management."))
            .unwrap();
        assert!(own.contains("## What to do\n\nAdapt this canvas for senior management.\n\n## The right block for each kind of information") && own.contains("How to change the board:") && !own.contains("Bring this canvas up to date"), "{own}");
        assert!(s
            .rethink_prompt(None, "", Some("  "))
            .unwrap()
            .contains(prompt::RETHINK_DEFAULT));
        // a board with perspectives: Rethink keeps each one filled
        assert!(!own.contains("## Perspectives"), "a sketch has none");
        let mut typed = Session::new();
        typed.set_source("type: project\n\napi: Orders API [tags: tech]\nbeta: Open beta\n");
        let p = typed.rethink_prompt(None, "", None).unwrap();
        assert!(
            p.contains("## Perspectives")
                && p.contains("- Technical: tag `tech`, shows entity, sequence, lanes, adr, cycle blocks whole; now shows 1 elements")
                && p.contains("- Delivery: tag `delivery`")
                && p.contains("none is empty"),
            "{p}"
        );
        // a resolved comment is not asked about
        let mut done = Session::new();
        done.set_source("a: A\ncomment on a: fine [resolved]\n");
        assert!(!done
            .rethink_prompt(None, "", None)
            .unwrap()
            .contains("## Open comments"));
        assert_eq!(
            prompt::reply_note("Sure.\n```json\n{\"note\": \" Grouped it. \", \"ops\": []}\n```")
                .as_deref(),
            Some("Grouped it.")
        );
        assert_eq!(prompt::reply_note("{\"ops\": []}"), None);
        assert_eq!(
            s.apply("{\"note\": \"Fine as it is.\", \"ops\": []}")["ok"],
            true
        );
    }

    #[test]
    fn changes_are_counted_in_every_canvas() {
        let old = parse("a: A\nb: B\npay: P [canvas: c]\n\ncanvas c: C {\n  x: X\n}\n").unwrap();
        let new =
            parse("a: A changed\npay: P [canvas: c]\nn: New\n\ncanvas c: C {\n  x: X\n  y: Y\n}\n")
                .unwrap();
        // a changed, b removed, n added; y added inside the canvas
        assert_eq!(change_count(&old, &new), 4);
        assert_eq!(change_count(&new, &new), 0);
    }

    #[test]
    fn in_ops_change_a_canvas_from_the_top() {
        let mut s = Session::new();
        s.set_source("pay: Payments [canvas: payments]\n\ncanvas payments: Payments {\n  intent: Create intent\n  comment on intent: split it\n  canvas fraud: Fraud {\n    rules\n  }\n}\n");
        let r = s.apply(r#"{"ops":[{"op":"update_node","id":"pay","color":"red"},{"op":"in","canvas":"payments","ops":[{"op":"update_node","id":"intent","label":"Create a payment intent"},{"op":"resolve_comment","id":"c1","resolution":"renamed"}]},{"op":"in","canvas":"payments/fraud","ops":[{"op":"add_node","id":"velocity","label":"Velocity check"}]}]}"#);
        assert_eq!(r["ok"], true, "{r}");
        let out = s.source("dsl").unwrap();
        assert!(
            out.contains("intent: Create a payment intent")
                && out.contains("velocity: Velocity check")
                && out.contains("pay: Payments [red, canvas: payments]"),
            "{out}"
        );
        // errors say where
        let e = s.apply(r#"{"op":"in","canvas":"payments","ops":[{"op":"update_node","id":"intnt","label":"x"}]}"#);
        assert!(
            e["error"]
                .as_str()
                .unwrap()
                .contains("in canvas `payments`, op #1: no node `intnt` (did you mean `intent`?)"),
            "{e}"
        );
        assert!(s.apply(r#"{"op":"in","canvas":"nope","ops":[]}"#)["error"]
            .as_str()
            .unwrap()
            .contains("no canvas `nope`"));
        assert!(s.apply(r#"{"op":"in","canvas":"","ops":[]}"#)["ok"] == false);
    }

    #[test]
    fn canvases_open_edit_and_save_in_place() {
        let src = "title: Checkout\n\n# the way in\npay: Payments [pill, canvas: payments]\napi -> pay\n\ncanvas payments: Payments flow {\n  # steps\n  intent: Create intent\n\n  canvas fraud: Fraud {\n    rules\n  }\n}\n";
        let mut s = Session::new();
        assert_eq!(s.set_source(src)["ok"], true, "{:?}", s.errors);
        assert_eq!(s.source("dsl").unwrap(), src);
        // inside the canvas: ops and prompts are about it
        s.open_canvas(&["payments".into()]).unwrap();
        assert!(s.doc.node("intent").is_some() && s.doc.node("pay").is_none());
        assert_eq!(
            s.apply(r#"{"op":"add_node","id":"confirm","label":"Confirm card"}"#)["ok"],
            true
        );
        assert!(prompt::brief(&s.doc).contains("Confirm card"));
        let out = s.source("dsl").unwrap();
        assert!(
            out.starts_with(
                "title: Checkout\n\n# the way in\npay: Payments [pill, canvas: payments]\n"
            ),
            "{out}"
        );
        assert!(out.contains("canvas payments: Payments flow {\n  # steps\n  intent: Create intent\n  confirm: Confirm card\n"), "{out}");
        assert_eq!(dsl::parse(&out).unwrap(), s.whole());
        // deeper, and back up through the trail
        s.open_canvas(&["payments".into(), "fraud".into()]).unwrap();
        let trail: Vec<String> = s.canvas_trail().into_iter().map(|t| t.1).collect();
        assert_eq!(trail, ["Checkout", "Payments flow", "Fraud"]);
        assert!(s.open_canvas(&["nope".into()]).is_err());
        s.open_canvas(&[]).unwrap();
        assert!(s.doc.node("pay").is_some());
        // what a model reads of the top board leaves the canvas out
        let lean = dsl::print_lean(&s.doc);
        assert!(
            !lean.contains("intent") && lean.contains("payments (“Payments flow”, 4 elements)"),
            "{lean}"
        );
        // removing the link removes the canvas
        assert_eq!(s.apply(r#"{"op":"remove","id":"pay"}"#)["ok"], true);
        assert!(s.doc.canvases.is_empty());
        // a new canvas and a box that opens it; removing the canvas removes its links
        let r = s.apply(r#"{"ops":[{"op":"add_canvas","title":"Refunds"},{"op":"add_node","id":"r","label":"Refunds","canvas":"refunds"}]}"#);
        assert_eq!(r["created"], json!(["refunds", "r"]));
        assert!(s
            .source("dsl")
            .unwrap()
            .ends_with("canvas refunds: Refunds {}\n"));
        assert_eq!(s.apply(r#"{"op":"remove","id":"refunds"}"#)["ok"], true);
        assert!(s.doc.node("r").is_none());
        // a link to a canvas that is not there is reported
        assert!(dsl::parse("x [canvas: gone]\n").is_err());
    }

    #[test]
    fn a_projects_canvas_has_a_summary_of_its_own() {
        let mut s = Session::new();
        s.set_source(
            "web_link: Web shop [canvas: web]\napp_link: Mobile app [canvas: app]\n\n\
             canvas web: Web shop {\n  canvas tests: Test scenarios {\n    a: A\n  }\n}\n\n\
             canvas app: Mobile app {\n  b: B\n}\n",
        );
        let today = gantt::parse_date("2026-10-15").unwrap();
        assert!(s.summary_path(today).is_empty());
        s.open_canvas(&["web".into()]).unwrap();
        assert_eq!(s.summary_path(today), ["web"]);
        // a canvas inside a project is part of its summary
        s.open_canvas(&["web".into(), "tests".into()]).unwrap();
        assert_eq!(s.summary_path(today), ["web"]);
        // one project's canvases are part of the whole board's
        let mut one = Session::new();
        one.set_source("a: A [canvas: tests]\n\ncanvas tests: Tests {\n  b: B\n}\n");
        one.open_canvas(&["tests".into()]).unwrap();
        assert!(one.summary_path(today).is_empty());
    }

    #[test]
    fn session_keeps_good_parts_on_errors() {
        let mut s = Session::new();
        let r = s.set_source("a -> b\nc [shape: blob]");
        assert_eq!(r["ok"], false);
        assert_eq!(s.doc.nodes.len(), 3);
        let r = s.apply(r#"{"ops":[{"op":"add_edge","from":"b","to":"c"}]}"#);
        assert_eq!(r["ok"], true, "{r}");
        s.set_base(Some("a -> b"));
        let out = s.render(&RenderOptions::default());
        assert!(
            out.svg.contains(theme::LIGHT.add),
            "added elements are green"
        );
        assert!(s.diff_markdown().contains("+ edge `b` → `c`"));
    }

    #[test]
    fn diffed_groups_are_valid_svg() {
        let old = parse("a -> b").unwrap();
        let new = parse("a -> b\ngroup g: G {\n c\n}").unwrap();
        let svg = render_diff(&old, &new, &RenderOptions::default()).svg;
        for tag in svg.split('<') {
            assert!(
                tag.matches(" stroke-width=").count() <= 1,
                "duplicate attribute in <{tag}"
            );
        }
    }
}
