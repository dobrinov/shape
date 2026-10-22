//! WebAssembly bindings. Everything crosses the boundary as strings (JSON
//! for structured values) so the JS glue stays tiny and dependency free.
use serde_json::{json, Value};
use shape_core::prompt::PromptOptions;
use shape_core::{RenderOptions, Session};
use wasm_bindgen::prelude::*;

fn to_json<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|e| json!({ "error": e.to_string() }).to_string())
}

fn opts<T: serde::de::DeserializeOwned + Default>(s: &str) -> Result<T, String> {
    if s.trim().is_empty() {
        Ok(T::default())
    } else {
        serde_json::from_str(s).map_err(|e| format!("invalid options: {e}"))
    }
}

fn err(e: impl std::fmt::Display) -> String {
    json!({ "error": e.to_string() }).to_string()
}

fn text_result(r: Result<String, impl std::fmt::Display>) -> String {
    match r {
        Ok(text) => json!({ "ok": true, "text": text }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
    .to_string()
}

/// A document being edited.
#[wasm_bindgen]
pub struct Diagram {
    inner: Session,
}

#[wasm_bindgen]
impl Diagram {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Diagram {
        Diagram {
            inner: Session::new(),
        }
    }

    /// Load DSL, JSON or Mermaid. Returns `{ok, format, errors: [{line, message}]}`.
    pub fn set_source(&mut self, src: &str) -> String {
        to_json(&self.inner.set_source(src))
    }

    /// The document as `dsl`, `json`, `mermaid`, `brief` or `svg`.
    pub fn source(&self, format: &str) -> String {
        self.inner
            .source(format)
            .unwrap_or_else(|e| format!("error: {e}"))
    }

    /// Like `source`, with errors kept apart: `{ok, text}` or `{ok: false, error}`.
    pub fn source_result(&self, format: &str) -> String {
        text_result(self.inner.source(format))
    }

    /// Open the canvas at `path` (a JSON array of canvas ids from the top
    /// board; `[]`: the top). Ops, drawing and prompts are then about it;
    /// `source` is still the whole board. Returns `{ok}` or `{ok: false, error}`.
    pub fn open_canvas(&mut self, path: &str) -> String {
        let path: Vec<String> = serde_json::from_str(path).unwrap_or_default();
        match self.inner.open_canvas(&path) {
            Ok(()) => json!({"ok": true}).to_string(),
            Err(e) => json!({"ok": false, "error": e}).to_string(),
        }
    }

    /// Where the open canvas is: `[{path, title}]`, the top board first.
    pub fn canvas_trail(&self) -> String {
        let t: Vec<Value> = self
            .inner
            .canvas_trail()
            .into_iter()
            .map(|(path, title)| json!({"path": path, "title": title}))
            .collect();
        Value::Array(t).to_string()
    }

    /// Replace the open canvas with a document (JSON); the rest of the board
    /// stays as written. Returns `{ok}` or `{ok: false, error}`.
    pub fn replace_here(&mut self, json: &str) -> String {
        match serde_json::from_str::<shape_core::Document>(json) {
            Ok(d) => {
                self.inner.replace_here(d);
                json!({"ok": true}).to_string()
            }
            Err(e) => json!({"ok": false, "error": e.to_string()}).to_string(),
        }
    }

    /// The document model as JSON.
    pub fn document(&self) -> String {
        to_json(&self.inner.doc)
    }

    pub fn errors(&self) -> String {
        to_json(&self.inner.errors)
    }

    /// Apply ops to the top board, also while a canvas is open (the board's
    /// type, its perspectives). Returns like `apply`.
    pub fn apply_top(&mut self, ops: &str) -> String {
        to_json(&self.inner.apply_top(ops))
    }

    /// Apply `{"ops": [...]}` (or an LLM reply containing it).
    /// Returns `{ok, created, count}` or `{ok: false, error, index}`.
    pub fn apply(&mut self, ops: &str) -> String {
        to_json(&self.inner.apply(ops))
    }

    /// Render with JSON options `{theme, comments: "open"|"all"|"none", title, transparent, padding}`.
    /// Returns `{svg, view, nodes, groups, edges, pins}`.
    pub fn render(&self, options: &str) -> String {
        match opts::<RenderOptions>(options) {
            Ok(o) => to_json(&self.inner.render(&o)),
            Err(e) => err(e),
        }
    }

    /// A theme file the board names (`theme: ./brand.theme`): its text, under
    /// that name. Returns `{ok}` or `{error}`.
    pub fn add_theme_file(&mut self, name: &str, src: &str) -> String {
        match self.inner.add_theme_file(name, src) {
            Ok(()) => json!({"ok": true}).to_string(),
            Err(e) => err(e),
        }
    }

    /// A theme as text: the board's own, or a preset ("" if neither).
    pub fn theme_text(&self, name: &str) -> String {
        let t = self
            .inner
            .doc
            .themes
            .iter()
            .find(|t| t.id == name)
            .cloned()
            .or_else(|| shape_core::theme::preset(name));
        t.map(|t| shape_core::dsl::theme_text(&t))
            .unwrap_or_default()
    }

    /// The presets' names, and the themes this board has.
    pub fn themes(&self) -> String {
        let presets: Vec<&str> = shape_core::theme::PRESETS.iter().map(|(n, _)| *n).collect();
        let own: Vec<&str> = self
            .inner
            .doc
            .themes
            .iter()
            .map(|t| t.id.as_str())
            .collect();
        json!({"presets": presets, "board": own, "current": self.inner.doc.settings.palette})
            .to_string()
    }

    /// Diff against a base version (any format); `undefined` clears it.
    pub fn set_base(&mut self, src: Option<String>) -> String {
        to_json(&self.inner.set_base(src.as_deref()))
    }

    pub fn diff(&self) -> String {
        match &self.inner.base {
            Some(b) => to_json(&shape_core::diff::diff(b, &self.inner.doc)),
            None => to_json(&json!({ "entries": [] })),
        }
    }

    pub fn diff_markdown(&self) -> String {
        self.inner.diff_markdown()
    }

    /// A prompt from the review comments. Options:
    /// `{target: "diagram"|"code"|"discuss", reply: "ops"|"dsl", comments: [ids], include_resolved, instructions, file}`.
    pub fn prompt(&self, options: &str) -> String {
        match opts::<PromptOptions>(options) {
            Ok(o) => self.inner.prompt(&o),
            Err(e) => format!("error: {e}"),
        }
    }

    /// Like `prompt`, with errors kept apart: `{ok, text}` or `{ok: false, error}`.
    pub fn prompt_result(&self, options: &str) -> String {
        text_result(opts::<PromptOptions>(options).map(|o| self.inner.prompt(&o)))
    }

    pub fn brief(&self) -> String {
        shape_core::prompt::brief(&self.inner.doc)
    }

    /// The mindmap under `id` as an indented outline (with `id` as the first line).
    pub fn outline(&self, id: &str) -> String {
        shape_core::mindmap::outline(&self.inner.doc, id, true)
    }

    /// The mindmap under `id` as Markdown (title, sections, bullets): a
    /// prompt, followed by what its `#id` links point at.
    pub fn markdown(&self, id: &str) -> String {
        if self.inner.doc.node(id).is_none() {
            return String::new();
        }
        shape_core::prompt::selection(&self.inner.doc, &[id.to_string()])
    }

    /// The selected items (a JSON array of ids) as Markdown to paste into a
    /// prompt: sections with their blocks, arrows between them, open comments.
    pub fn selection_markdown(&self, ids: &str) -> String {
        let ids: Vec<String> = serde_json::from_str(ids).unwrap_or_default();
        shape_core::prompt::selection(&self.inner.doc, &ids)
    }

    /// The selected items as Markdown, with gantt forecasts as of `today`
    /// (`YYYY-MM-DD`; empty: none).
    /// The selection as text for people (Copy as text): what the board
    /// shows, without details or ids. `today` as `YYYY-MM-DD`, or "".
    pub fn selection_text_at(&self, ids: &str, today: &str) -> String {
        let ids: Vec<String> = serde_json::from_str(ids).unwrap_or_default();
        let t = shape_core::gantt::parse_date(today);
        shape_core::prompt::plain_text(&self.inner.doc, &ids, t)
    }

    pub fn selection_markdown_at(&self, ids: &str, today: &str) -> String {
        let ids: Vec<String> = serde_json::from_str(ids).unwrap_or_default();
        let t = shape_core::gantt::parse_date(today);
        shape_core::prompt::selection_at(&self.inner.doc, &ids, t)
    }

    /// The whole board, every canvas in it, as a prompt to build what it
    /// describes (gantt forecasts as of `today`, `YYYY-MM-DD`).
    pub fn build_prompt(&self, today: &str) -> String {
        shape_core::prompt::build(&self.inner.whole(), shape_core::gantt::parse_date(today))
    }

    /// The brief, with gantt forecasts as of `today` (`YYYY-MM-DD`).
    pub fn brief_at(&self, today: &str) -> String {
        shape_core::prompt::brief_at(&self.inner.doc, shape_core::gantt::parse_date(today))
    }

    /// Every gantt chart's tasks, planned against the forecast as of `today`
    /// (`YYYY-MM-DD`): `{today, charts: [{id, label, tasks: [{id, start, end,
    /// days, done, milestone, forecast_start, forecast_end, slip, status}]}]}`.
    pub fn schedule(&self, today: &str) -> String {
        let Some(t) = shape_core::gantt::parse_date(today) else {
            return err(format!("`{today}` is not a date (YYYY-MM-DD)"));
        };
        let doc = &self.inner.doc;
        let charts: Vec<Value> = doc
            .groups
            .iter()
            .filter(|g| g.gantt)
            .map(|g| json!({"id": g.id, "label": g.display(), "tasks": shape_core::gantt::track(doc, &g.id, t)}))
            .collect();
        json!({"today": shape_core::gantt::format_date(t), "charts": charts}).to_string()
    }

    /// Part of the board as Shape language: `selectors` is a JSON array
    /// (`["group:back", "near:api"]`, see `shape help`). Returns
    /// `{source, ids}` or `{error}`.
    pub fn query(&self, selectors: &str) -> String {
        let sels: Vec<String> = match serde_json::from_str(selectors) {
            Ok(s) => s,
            Err(e) => return err(format!("selectors: {e}")),
        };
        let sels: Vec<&str> = sels.iter().map(|s| s.as_str()).collect();
        match shape_core::query::select(&self.inner.doc, &sels) {
            Ok(ids) => json!({
                "source": shape_core::dsl::print(&shape_core::query::subset(&self.inner.doc, &ids)),
                "ids": ids,
            })
            .to_string(),
            Err(e) => err(e),
        }
    }

    /// A prompt asking a model to change the board as `request` says, about
    /// the selected `ids` (a JSON array; empty: the whole board).
    pub fn ask_prompt(&self, ids: &str, request: &str) -> String {
        let ids: Vec<String> = serde_json::from_str(ids).unwrap_or_default();
        shape_core::prompt::ask(&self.inner.doc, &ids, request, &self.inner.recipes)
    }

    /// What this version of Shape can do with the board since it was last
    /// rethought, for its built-in perspectives: `{version, rethought,
    /// notes: [short, …]}` (no notes: nothing new for it).
    pub fn upgrade_notes(&self) -> String {
        let root = self.inner.whole();
        json!({
            "version": shape_core::upgrades::VERSION,
            "rethought": root.settings.rethought,
            "notes": shape_core::upgrades::since(&root).iter().map(|n| n.short).collect::<Vec<_>>(),
        })
        .to_string()
    }

    /// A prompt asking a model to draft test scenarios for the open canvas
    /// (and the canvases inside it), from how the board says it works.
    pub fn tests_prompt(&self) -> String {
        shape_core::qa::draft_prompt(&self.inner.doc)
    }

    /// The ops that put a [`tests_prompt`](Self::tests_prompt) reply on the
    /// open canvas: `{ops}` (to `apply`) or `{error}`.
    pub fn tests_ops(&self, reply: &str) -> String {
        match shape_core::qa::draft_ops(&self.inner.doc, reply) {
            Ok(ops) => ops,
            Err(e) => err(e),
        }
    }

    /// A prompt asking `question` about the whole board (canvases included).
    /// `history`: the conversation so far, `[[question, answer], …]` (JSON);
    /// `today`: `YYYY-MM-DD` for gantt forecasts ("" for none).
    pub fn question_prompt(&self, history: &str, question: &str, today: &str) -> String {
        let history: Vec<(String, String)> = serde_json::from_str(history).unwrap_or_default();
        self.inner
            .question_prompt(&history, question, shape_core::gantt::parse_date(today))
    }

    /// A team's own recipes, as `{id: file text}` (each starting `# recipe:
    /// Name — when`); prompts list them with the built-in ones.
    pub fn set_recipes(&mut self, files: &str) {
        let files: std::collections::BTreeMap<String, String> =
            serde_json::from_str(files).unwrap_or_default();
        self.inner.recipes = files
            .iter()
            .filter_map(|(id, text)| shape_core::recipes::from_file(id, text))
            .collect();
    }

    /// Every recipe (built in, then the team's): `[{id, name, when, shape, source}]`.
    pub fn recipes(&self) -> String {
        to_json(
            &shape_core::recipes::builtins()
                .into_iter()
                .chain(self.inner.recipes.iter().cloned())
                .collect::<Vec<_>>(),
        )
    }

    /// To-do list `gid`'s items as text, one per line ("" when it is not one).
    pub fn todo_text(&self, gid: &str) -> String {
        shape_core::todo::text_of(&self.inner.doc, gid).unwrap_or_default()
    }

    /// The ops that make to-do list `gid` what `text` says: `{ok, ops}` or
    /// `{ok: false, error}`.
    pub fn todo_edit_ops(&self, gid: &str, text: &str) -> String {
        match shape_core::todo::edit_ops(&self.inner.doc, gid, text) {
            Ok(ops) => serde_json::json!({ "ok": true, "ops": ops }).to_string(),
            Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
        }
    }

    /// The ops that bring schema block `gid` up to date with `src` (its
    /// file's text): `{ok, ops, summary, added, changed, removed, warnings}`
    /// or `{ok: false, error}`.
    pub fn schema_sync_ops(&self, gid: &str, src: &str) -> String {
        sync_json(shape_core::schemas::sync_ops(&self.inner.doc, gid, src))
    }

    /// The ops that add a schema block with the tables in `src`; `file`
    /// (relative to the board) and `only` (globs) may be "".
    pub fn schema_add_ops(&self, label: &str, file: &str, only: &str, src: &str) -> String {
        let some = |s: &str| Some(s.to_string()).filter(|s| !s.is_empty());
        sync_json(shape_core::schemas::add_ops(
            &self.inner.doc,
            label,
            some(file).as_deref(),
            some(only).as_deref(),
            src,
        ))
    }

    /// Forecast `gid`'s lines as text ("" when it is not a forecast).
    pub fn forecast_text(&self, gid: &str) -> String {
        shape_core::forecast::text_of(&self.inner.doc, gid).unwrap_or_default()
    }

    /// The ops that make forecast `gid` what `text` says: `{ok, ops}` or
    /// `{ok: false, error}`.
    pub fn forecast_edit_ops(&self, gid: &str, text: &str) -> String {
        match shape_core::forecast::edit_ops(&self.inner.doc, gid, text) {
            Ok(ops) => serde_json::json!({ "ok": true, "ops": ops }).to_string(),
            Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
        }
    }

    /// Screen `gid`'s components as text, one per line ("" when it is not
    /// a screen).
    pub fn screen_text(&self, gid: &str) -> String {
        shape_core::screen::text_of(&self.inner.doc, gid).unwrap_or_default()
    }

    /// The ops that make screen `gid` what `text` says: `{ok, ops}` or
    /// `{ok: false, error}`.
    pub fn screen_edit_ops(&self, gid: &str, text: &str) -> String {
        match shape_core::screen::edit_ops(&self.inner.doc, gid, text) {
            Ok(ops) => serde_json::json!({ "ok": true, "ops": ops }).to_string(),
            Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
        }
    }

    /// What the board is for (`sketch`, `product`, `project`), from its top
    /// board, also while a canvas is open.
    pub fn board_type(&self) -> String {
        self.inner.whole().settings.kind.as_str().to_string()
    }

    /// The board's perspectives: `[{id, label, tags, show, details}]`, the
    /// built-in ones of its type first. Render with one as `perspective`.
    pub fn perspectives(&self) -> String {
        to_json(&self.inner.perspectives())
    }

    /// A prompt asking `question` about the whole board, answered with a
    /// small board of the model's own. `history`: `[[question, summary], …]`.
    pub fn answer_board_prompt(&self, history: &str, question: &str, today: &str) -> String {
        let history: Vec<(String, String)> = serde_json::from_str(history).unwrap_or_default();
        self.inner
            .answer_board_prompt(&history, question, shape_core::gantt::parse_date(today))
    }

    /// The places an answer points at: `[{ref, path, id, label}]` (label
    /// null when nothing on the board has that id).
    pub fn answer_refs(&self, answer: &str) -> String {
        to_json(&self.inner.answer_refs(answer))
    }

    /// A prompt to rethink the whole board from the open canvas: act on the
    /// open comments, organise what was collected. `since` is an earlier
    /// version of the whole board (null: none; "": the board did not exist),
    /// whose changes are new information; `label` says what it is. Its ops
    /// apply to the top board. `instructions` replace the default ones
    /// ([`rethink_default`]). Returns `{prompt}` or `{error}`.
    pub fn rethink_prompt(
        &self,
        since: Option<String>,
        label: &str,
        instructions: Option<String>,
    ) -> String {
        match self
            .inner
            .rethink_prompt(since.as_deref(), label, instructions.as_deref())
        {
            Ok(p) => json!({ "prompt": p }).to_string(),
            Err(e) => err(e),
        }
    }

    /// A small prompt asking a model to grow the topic `id`.
    /// Options: `{instructions, count, levels}`. Returns `{prompt}` or `{error}`.
    pub fn expand_prompt(&self, id: &str, options: &str) -> String {
        let o: shape_core::mindmap::ExpandOptions = match opts(options) {
            Ok(o) => o,
            Err(e) => return err(e),
        };
        match shape_core::mindmap::expand_prompt(&self.inner.doc, id, &o) {
            Ok(p) => json!({ "prompt": p }).to_string(),
            Err(e) => err(e),
        }
    }
}

/// The three versions of a merge as boards ("" for an unknown base).
fn merge_docs(base: &str, ours: &str, theirs: &str) -> Result<[shape_core::Document; 3], String> {
    let read = |what: &str, s: &str| -> Result<shape_core::Document, String> {
        let s = shape_core::merge::strip_note(s);
        if s.trim().is_empty() {
            return Ok(Default::default());
        }
        shape_core::parse(s).map_err(|e| {
            format!(
                "{what}: {}",
                e.iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    };
    Ok([
        read("base", base)?,
        read("yours", ours)?,
        read("theirs", theirs)?,
    ])
}

fn picks_of(picks: &str) -> std::collections::BTreeMap<String, String> {
    serde_json::from_str(picks).unwrap_or_default()
}

/// Merge two versions of a board changed from `base`: `{ok, source,
/// conflicts: [{key, kind, id, label, canvas, fields, text}]}` (source keeps
/// yours where they clash), or `{ok: false, error}`.
#[wasm_bindgen]
pub fn merge3(base: &str, ours: &str, theirs: &str) -> String {
    let [b, o, t] = match merge_docs(base, ours, theirs) {
        Ok(d) => d,
        Err(e) => return json!({"ok": false, "error": e}).to_string(),
    };
    let m = shape_core::merge::merge3(&b, &o, &t);
    let src = shape_core::patch::patch(shape_core::merge::strip_note(ours), &o, &m.doc);
    let conflicts: Vec<Value> = m
        .conflicts
        .iter()
        .map(|c| {
            json!({"key": c.key(), "kind": c.kind, "id": c.id, "label": c.label, "canvas": c.canvas,
                   "fields": c.fields, "text": shape_core::merge::describe(c),
                   "ours": c.ours.is_some(), "theirs": c.theirs.is_some()})
        })
        .collect();
    json!({"ok": true, "source": src, "conflicts": conflicts}).to_string()
}

/// The board after picking sides (`picks`: `{"node:api": "theirs"}`) and
/// then applying `ops` (a JSON array, may be empty): `{ok, source}` or
/// `{ok: false, error}`.
#[wasm_bindgen]
pub fn merge_resolve(base: &str, ours: &str, theirs: &str, picks: &str, ops: &str) -> String {
    let [b, o, t] = match merge_docs(base, ours, theirs) {
        Ok(d) => d,
        Err(e) => return json!({"ok": false, "error": e}).to_string(),
    };
    let m = shape_core::merge::merge3(&b, &o, &t);
    let mut doc = shape_core::merge::resolve(&m, &picks_of(picks));
    let ops: Vec<Value> = serde_json::from_str(ops).unwrap_or_default();
    if !ops.is_empty() {
        let parsed = match shape_core::ops::parse_ops(&json!({ "ops": ops }).to_string()) {
            Ok(p) => p,
            Err(e) => return json!({"ok": false, "error": e}).to_string(),
        };
        if let Err(e) = shape_core::ops::apply(&mut doc, &parsed) {
            return json!({"ok": false, "error": e.to_string()}).to_string();
        }
    }
    let src = shape_core::patch::patch(shape_core::merge::strip_note(ours), &o, &doc);
    json!({"ok": true, "source": src}).to_string()
}

/// One version of a merge drawn: `side` is
/// `ours`, `theirs` or `merged` (with `picks`). `{svg, view}` or `{error}`.
#[wasm_bindgen]
pub fn merge_svg(
    base: &str,
    ours: &str,
    theirs: &str,
    side: &str,
    picks: &str,
    options: &str,
) -> String {
    let [b, o, t] = match merge_docs(base, ours, theirs) {
        Ok(d) => d,
        Err(e) => return err(e),
    };
    let m = shape_core::merge::merge3(&b, &o, &t);
    let doc = match side {
        "ours" => o,
        "theirs" => t,
        _ => shape_core::merge::resolve(&m, &picks_of(picks)),
    };
    // drawn as they are: the editor outlines what clashed
    let opts: RenderOptions = opts(options).unwrap_or_default();
    to_json(&shape_core::render::render(&doc, &opts))
}

/// A prompt asking a model to settle the conflicts as `instruction` says.
#[wasm_bindgen]
pub fn merge_prompt(base: &str, ours: &str, theirs: &str, instruction: &str) -> String {
    match merge_docs(base, ours, theirs) {
        Ok([b, o, t]) => {
            shape_core::merge::prompt(&shape_core::merge::merge3(&b, &o, &t), instruction)
        }
        Err(e) => format!("error: {e}"),
    }
}

/// A model's answer to `merge_prompt`: `{ok, picks, ops}` or `{ok: false, error}`.
#[wasm_bindgen]
pub fn merge_answer(reply: &str) -> String {
    match shape_core::merge::parse_answer(reply) {
        Ok((picks, ops)) => json!({"ok": true, "picks": picks, "ops": ops}).to_string(),
        Err(e) => json!({"ok": false, "error": e}).to_string(),
    }
}

/// A file git merged line by line: `{base, ours, theirs}` (base null when
/// git did not write it), or null without conflict markers.
#[wasm_bindgen]
pub fn split_markers(text: &str) -> String {
    match shape_core::merge::split_markers(text) {
        Some((b, o, t)) => json!({"base": b, "ours": o, "theirs": t}).to_string(),
        None => "null".into(),
    }
}

/// How many elements changed from board `old` to board `new` (any format),
/// canvases included; -1 when either does not parse. "" for `old`: none existed.
#[wasm_bindgen]
pub fn count_changes(old: &str, new: &str) -> i32 {
    let old = if old.trim().is_empty() {
        Ok(Default::default())
    } else {
        shape_core::parse(old)
    };
    match (old, shape_core::parse(new)) {
        (Ok(o), Ok(n)) => shape_core::change_count(&o, &n) as i32,
        _ => -1,
    }
}

/// A model's answer board: `{ok, summary, source}` or `{ok: false, error}`.
#[wasm_bindgen]
pub fn answer_parts(reply: &str) -> String {
    match shape_core::prompt::answer_parts(reply) {
        Ok((summary, source)) => {
            json!({"ok": true, "summary": summary, "source": source}).to_string()
        }
        Err(e) => json!({"ok": false, "error": e}).to_string(),
    }
}

/// What built-in perspective `id` is for, in a line ("" for a board's own).
#[wasm_bindgen]
pub fn perspective_purpose(id: &str) -> String {
    shape_core::perspectives::purpose(id)
        .unwrap_or("")
        .to_string()
}

/// The names of the perspectives a board of type `kind` comes with (JSON).
#[wasm_bindgen]
pub fn builtin_perspectives(kind: &str) -> String {
    let kind = shape_core::model::BoardKind::parse(kind).unwrap_or_default();
    to_json(
        &shape_core::perspectives::builtins(kind)
            .into_iter()
            .map(|p| p.label)
            .collect::<Vec<_>>(),
    )
}

/// What Rethink does unless told otherwise (its editable instructions).
#[wasm_bindgen]
pub fn rethink_default() -> String {
    shape_core::prompt::RETHINK_DEFAULT.to_string()
}

/// The note in a model's reply to a rethink prompt, or "".
#[wasm_bindgen]
pub fn reply_note(reply: &str) -> String {
    shape_core::prompt::reply_note(reply).unwrap_or_default()
}

/// The prompt again after a reply that could not be applied.
#[wasm_bindgen]
pub fn ask_again(prompt: &str, reply: &str, error: &str) -> String {
    shape_core::prompt::ask_again(prompt, reply, error)
}

/// A built-in theme's text (`paper`, `blueprint`, `midnight`, `contrast`), or "".
#[wasm_bindgen]
pub fn theme_preset(name: &str) -> String {
    shape_core::theme::PRESETS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| s.to_string())
        .unwrap_or_default()
}

/// A prompt asking a model for a new mindmap about `topic`.
#[wasm_bindgen]
pub fn mindmap_prompt(topic: &str, options: &str) -> String {
    let o: shape_core::mindmap::ExpandOptions = opts(options).unwrap_or_default();
    shape_core::mindmap::generate_prompt(topic, &o)
}

/// A model's reply as an outline (two spaces per level), ready for `add_outline`.
#[wasm_bindgen]
pub fn reply_outline(reply: &str) -> String {
    shape_core::mindmap::parse_reply(reply)
        .iter()
        .map(|l| format!("{}{}\n", "  ".repeat(l.depth), l.text))
        .collect()
}

impl Default for Diagram {
    fn default() -> Self {
        Self::new()
    }
}

/// One-shot: source → `{svg, view, …}` or `{error}`.
#[wasm_bindgen]
pub fn render(src: &str, options: &str) -> String {
    let o: RenderOptions = match opts(options) {
        Ok(o) => o,
        Err(e) => return err(e),
    };
    match shape_core::parse(src) {
        Ok(doc) => to_json(&shape_core::render::render(&doc, &o)),
        Err(e) => to_json(
            &json!({ "error": e.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("\n"), "errors": e }),
        ),
    }
}

/// One-shot conversion: `{ok, text}` or `{ok: false, errors}`.
#[wasm_bindgen]
pub fn convert(src: &str, to: &str) -> String {
    let v: Value = match shape_core::parse(src) {
        Ok(doc) => match shape_core::export(&doc, to) {
            Ok(text) => json!({ "ok": true, "text": text }),
            Err(e) => json!({ "ok": false, "errors": [{ "line": 0, "message": e }] }),
        },
        Err(e) => json!({ "ok": false, "errors": e }),
    };
    v.to_string()
}

#[wasm_bindgen]
pub fn spec() -> String {
    shape_core::spec()
}

#[wasm_bindgen]
pub fn json_schema() -> String {
    shape_core::json_schema().to_string()
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn sync_json(r: Result<shape_core::schemas::Sync, String>) -> String {
    match r {
        Ok(s) => serde_json::json!({ "ok": true, "ops": s.ops, "summary": s.summary(), "added": s.added, "changed": s.changed, "removed": s.removed, "warnings": s.warnings }).to_string(),
        Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
    }
}
