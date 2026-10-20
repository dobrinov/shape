mod ai;
mod app;
mod assets;
mod export;
mod history;
mod png;
mod server;
mod skills;
mod update;
mod window;

use std::collections::{HashMap, HashSet};
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use shape_core::model::{Status, Theme};
use shape_core::ops::Op;
use shape_core::prompt::{PromptOptions, Reply, Target};
use shape_core::render::CommentMode;
use shape_core::{Document, RenderOptions};

pub const HELP: &str = r#"Shape — one board for the whole project, as text, with your AI

USAGE
  shape [FILE]                       open FILE in the editor, in its own window (created if
                                     missing; no FILE asks where to save a new board)
  shape new FILE [--template flow|architecture|board|product|project]   start a board from a template
                                     (product and project boards come with perspectives)
  --canvas ID[/ID…]                  with any command: work on that canvas inside the board
                                     (`canvas id: Title { … }`), e.g. brief, apply, svg

EXPORT
  shape svg  FILE [-o out.svg]       static SVG
  shape png  FILE [-o out.png]       PNG image (--scale N, default 2)
                                     svg / png --perspective ID: that perspective's dashboard
  shape html FILE [-o out.html]      self-contained interactive page (viewer + WASM)
    render options: --dark --transparent --title --comments open|all|none
                    --today | --as-of YYYY-MM-DD   mark that day on gantt charts and show the forecast
                    --focus ID,ID   crop to these elements (look at one block)
                    --base FILE|GIT_REF   highlight what changed since then
  shape convert FILE --to dsl|json|mermaid|brief [-o OUT]
                                     FILE may be DSL, JSON or Mermaid (detected)
  shape brief FILE [--today|--as-of DATE]     Markdown description for LLM context
                                     (with a date: gantt forecasts, late and behind tasks)

MINDMAPS
  shape outline FILE [ID] [--md]     a mindmap as an indented outline (--md: Markdown, ready as a prompt)
  shape expand  FILE ID [--ask TEXT] [--count N] [--levels N] [--print]
                                     grow topic ID with AI (prints the prompt with --print or without a harness)
  shape expand  FILE --new TOPIC [--ask TEXT] [--in SECTION]     a new mindmap about TOPIC
    AI: --ai claude | claude:MODEL | codex | gemini | CMD, or $SHAPE_AI (any command
        reading the prompt on stdin); default: the first of claude, codex, gemini installed

REVIEW
  shape comments FILE [--all] [--json]              list comment threads
  shape comment  FILE [--on ID] [--by NAME] TEXT    add a comment
  shape reply    FILE COMMENT_ID [--by NAME] TEXT
  shape resolve  FILE COMMENT_ID [--note TEXT]
  shape prompt   FILE [--target build|diagram|code|discuss] [--reply ops|dsl]
                 [--comment c1,c2] [--all] [--instructions TEXT]
                 [--context auto|near|section|all]   how much of the board to show
                 (auto: all of a small board, else the commented sections)
                                     turn open comments into a prompt for an agent
                                     (--target build: the whole board as a spec to build from)
  shape apply    FILE [PATCH|-|'{"op":…}'] [--dry-run]   apply ops JSON (a file, stdin, inline,
                                     or an LLM reply containing it)

PLANNING
  shape schedule FILE [CHART] [--as-of DATE] [--json]     gantt tasks planned vs forecast as of
                                     today (or DATE): late, behind, pushed back, how many days

DATABASE SCHEMAS
  shape schema FILE SCHEMA_FILE [--only 'users orders*'] [--label TEXT] [--into ID] [--dry-run]
                                     put a schema file's tables on the board (Postgres
                                     structure.sql, MySQL, SQLite, Rails schema.rb), or update them
  shape schema FILE                  update every schema block from its file

AI (your subscription's CLI: Claude Code, Codex, Gemini CLI)
  shape ask FILE "what to change" [--on ID,ID] [--dry-run] [--print]
                                     the model changes the board (ops; a reply that does not apply
                                     is sent back with its error, twice at most); --print: the prompt
    --ai claude | claude:sonnet | codex | gemini | CMD   (or $SHAPE_AI; default: the first installed)

FOR AGENTS
  shape query FILE SELECTOR...       just part of the board, as Shape language (--ids: ids only)
                                     ID · group:ID · chart:ID · map:ID · near:ID · label~TEXT
                                     kind:box|topic|task|milestone|sticky|decision|group|edge|comment
                                     owner:NAME · tag:NAME · color:NAME · comments:open
  shape add   FILE [DSL|-] [--dry-run]   add statements (new elements; one written again by its
                                     id gets the attributes given: `api [red]`)

CHECK
  shape check FILE...                validate (exit 1 on errors)
  shape fmt   FILE... [--check]      rewrite in canonical form
  shape diff  OLD NEW [--json]       what changed (NEW may be a git ref: shape diff FILE HEAD~1)
  shape merge BASE OURS THEIRS [-o OUT]     merge two versions element by element; lists conflicts
  shape merge --install              in this repository: git merges .shape (and .diagram) files with `shape merge`
                                     (a clash keeps yours and is marked; open the board to choose)

MORE
  shape spec [--json-schema]         the language reference (for people and LLMs)
  shape embed [-o shape.embed.js]    browser library bundle (JS + inlined WASM)
  shape skills list | show NAME | install [--global] [--dir PATH]     agent skills
                                     (installed ones follow upgrades, unless you edit them)
  shape install-app [--remove]       give .shape files their own icon and open them in the
                                     editor on a double-click (macOS, Linux, Windows)
  shape version

SERVER OPTIONS
  --port N  --ai CMD
  --browser                          open the editor in a browser tab instead of a window
  --no-open                          neither: just the server (no FILE then makes untitled.shape)
  --idle-exit                        stop once no editor tab has been heard from for 3 minutes

FILE may be `-` for stdin. Formats are chosen by extension: .shape or .diagram (DSL),
.json, .mmd (Mermaid). See https://github.com/dobrinov/shape
"#;

const VALUE_FLAGS: &[&str] = &[
    "-o",
    "--out",
    "--to",
    "--scale",
    "--target",
    "--reply",
    "--comment",
    "--comments",
    "--instructions",
    "--context",
    "--on",
    "--by",
    "--note",
    "--base",
    "--port",
    "--dir",
    "--template",
    "--theme",
    "--ai",
    "--ask",
    "--count",
    "--levels",
    "--new",
    "--focus",
    "--as-of",
    "--in",
    "--canvas",
    "--into",
    "--perspective",
    "--only",
    "--label",
];

struct Args {
    cmd: String,
    pos: Vec<String>,
    vals: HashMap<String, String>,
    flags: HashSet<String>,
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Args, String> {
        let mut pos = Vec::new();
        let mut vals = HashMap::new();
        let mut flags = HashSet::new();
        let mut it = raw.into_iter();
        while let Some(a) = it.next() {
            if let Some((k, v)) = a.split_once('=').filter(|(k, _)| k.starts_with("--")) {
                vals.insert(k.to_string(), v.to_string());
            } else if VALUE_FLAGS.contains(&a.as_str()) {
                let v = it.next().ok_or_else(|| format!("{a} needs a value"))?;
                vals.insert(if a == "-o" { "--out".into() } else { a }, v);
            } else if a.starts_with('-') && a != "-" {
                flags.insert(a);
            } else {
                pos.push(a);
            }
        }
        const COMMANDS: &[&str] = &[
            "new", "open", "svg", "png", "html", "convert", "brief", "comments", "comment",
            "reply", "resolve", "prompt", "apply", "check", "fmt", "diff", "spec", "embed",
            "skills", "version", "help", "outline", "expand", "query", "add", "schedule", "ask",
            "merge", "schema",
        ];
        let cmd = match pos.first() {
            Some(c) if COMMANDS.contains(&c.as_str()) || c == "install-app" => pos.remove(0),
            _ => "open".into(),
        };
        if vals.contains_key("--out") {
            vals.insert("-o".into(), vals["--out"].clone());
        }
        Ok(Args {
            cmd,
            pos,
            vals,
            flags,
        })
    }
    fn val(&self, k: &str) -> Option<&str> {
        self.vals.get(k).map(|s| s.as_str())
    }
    fn flag(&self, k: &str) -> bool {
        self.flags.contains(k)
    }
    fn file(&self, i: usize) -> Result<&str, String> {
        self.pos
            .get(i)
            .map(|s| s.as_str())
            .ok_or_else(|| format!("missing FILE (try `shape {} FILE`)", self.cmd))
    }
}

/// The first free `untitled.shape`, `untitled-2.shape`, … in the current directory.
fn untitled() -> String {
    (1..)
        .map(|n| match n {
            1 => "untitled.shape".to_string(),
            n => format!("untitled-{n}.shape"),
        })
        .find(|f| !Path::new(f).exists())
        .unwrap()
}

/// The first mindmap's centre.
fn first_map(doc: &Document) -> Result<String, String> {
    let f = shape_core::mindmap::Forest::new(doc);
    (0..doc.nodes.len())
        .find(|&i| f.root[i] == Some(i))
        .map(|i| doc.nodes[i].id.clone())
        .ok_or_else(|| "this diagram has no mindmap".to_string())
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match Args::parse(raw) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("shape: {e}");
            return ExitCode::from(2);
        }
    };
    if args.flag("-V") || args.flag("--version") {
        println!("shape {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    if args.flag("-h") || args.flag("--help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    if let Some(c) = args.val("--canvas").filter(|c| !c.is_empty()) {
        let _ = CANVAS.set(c.split('/').map(str::to_string).collect());
    }
    // after an upgrade, the installed Claude skills follow (unless edited)
    if args.cmd != "skills" {
        skills::refresh();
    }
    match run(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("shape: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn repo_root() -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

fn read_text(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        return Ok(s);
    }
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

/// A file's contents at a git ref (`HEAD`, `main`, `a1b2c3`, …).
fn git_show(reference: &str, file: &str) -> Result<String, String> {
    let abs = std::fs::canonicalize(file).map_err(|e| format!("{file}: {e}"))?;
    let dir = abs.parent().unwrap_or(Path::new("."));
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "--full-name"])
        .arg(&abs)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let rel = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if rel.is_empty() {
        return Err(format!("{file} is not tracked by git"));
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["show", &format!("{reference}:{rel}")])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git show {reference}:{rel}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Links to other boards (`delivery.shape#m1`) that lead nowhere: the board
/// is missing, or has no such element.
fn board_link_problems(doc: &Document, file: &str) -> Vec<String> {
    let dir = Path::new(file).parent().unwrap_or(Path::new(""));
    let mut out = Vec::new();
    for n in &doc.nodes {
        let Some((path, id)) = n.link.as_deref().and_then(shape_core::model::board_link) else {
            continue;
        };
        let Ok(src) = std::fs::read_to_string(dir.join(path)) else {
            out.push(format!("`{}` links to {path}, which does not exist", n.id));
            continue;
        };
        if let Some(id) = id {
            let (other, _) = shape_core::dsl::parse_lenient(&src);
            if other.node(id).is_none() && other.group(id).is_none() {
                out.push(format!(
                    "`{}` links to `{id}` on {path}, which has no `{id}`",
                    n.id
                ));
            }
        }
    }
    out
}

pub fn git_show_pub(reference: &str, file: &str) -> Result<String, String> {
    git_show(reference, file)
}

/// A team's recipes: `recipes/*.shape` (or `.diagram`) in the board's repository (or
/// beside it), each starting `# recipe: Name — when to use it`.
pub fn team_recipes(file: &str) -> Vec<shape_core::recipes::Recipe> {
    recipe_files(file)
        .iter()
        .filter_map(|(id, text)| shape_core::recipes::from_file(id, text))
        .collect()
}

/// The `recipes/*.shape` files for a board: `(id, text)`.
pub fn recipe_files(file: &str) -> Vec<(String, String)> {
    let start = std::fs::canonicalize(file)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let mut dir = start;
    let mut out = Vec::new();
    // from the board's folder up to the repository's root
    while let Some(d) = dir {
        let folder = d.join("recipes");
        if let Ok(entries) = std::fs::read_dir(&folder) {
            let mut files: Vec<_> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension()
                        .is_some_and(|x| x == "shape" || x == "diagram")
                })
                .collect();
            files.sort();
            for f in files {
                if let (Some(stem), Ok(text)) = (
                    f.file_stem().and_then(|s| s.to_str()),
                    std::fs::read_to_string(&f),
                ) {
                    if !out.iter().any(|(id, _): &(String, String)| id == stem) {
                        out.push((stem.to_string(), text));
                    }
                }
            }
        }
        if d.join(".git").exists() {
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    out
}

/// The version of `file` to rethink a board against, and what it is: on a
/// branch, where it left the main branch (the file's text then, "" when it
/// did not exist yet); on the main branch, the last commit. None when the
/// file is not in git (or not committed yet).
pub fn git_since(file: &str) -> Option<(String, String)> {
    git_since_on(file).map(|s| (s.0, s.1))
}

/// [`git_since`], with the branch this is on and the main branch
/// (`research`, `main`; on the main branch both are its name).
pub fn git_since_on(file: &str) -> Option<(String, String, String, String)> {
    let abs = std::fs::canonicalize(file).ok()?;
    let dir = abs.parent().unwrap_or(Path::new("."));
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    git(&["ls-files", "--error-unmatch", &abs.to_string_lossy()])?;
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    // the main branch: what origin's HEAD points at, else main or master
    let main = git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        .or_else(|| {
            ["main", "master"]
                .into_iter()
                .find(|b| git(&["rev-parse", "--verify", "--quiet", b]).is_some())
                .map(String::from)
        })
        .unwrap_or_default();
    let short = main.rsplit('/').next().unwrap_or("");
    if !main.is_empty() && branch != short && branch != "HEAD" {
        if let Some(base) = git(&["merge-base", "HEAD", &main]) {
            if git(&["rev-parse", "HEAD"]).as_deref() != Some(base.as_str()) {
                let label = format!("where the branch `{branch}` left `{short}`");
                return Some((
                    git_show(&base, file).unwrap_or_default(),
                    label,
                    branch.clone(),
                    short.to_string(),
                ));
            }
        }
    }
    git_show("HEAD", file).ok().map(|s| {
        let on = if branch.is_empty() || branch == "HEAD" {
            short.to_string()
        } else {
            branch.clone()
        };
        (s, "the last commit".to_string(), on.clone(), on)
    })
}

fn parse_doc(src: &str, what: &str) -> Result<Document, String> {
    shape_core::parse(src).map_err(|errs| {
        format!(
            "{what}:\n{}",
            errs.iter()
                .map(|e| format!("  {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })
}

/// `shape merge`: a three-way merge of boards. As git's merge driver
/// (`--driver`, `%O %A %B`) it writes the result over OURS and fails when
/// something clashed, so git leaves the file unmerged for the editor.
fn merge_cmd(args: &Args) -> Result<ExitCode, String> {
    if args.flag("--install") {
        let git = |a: &[&str]| {
            Command::new("git")
                .args(a)
                .status()
                .map_err(|e| format!("git: {e}"))
                .and_then(|s| {
                    s.success()
                        .then_some(())
                        .ok_or_else(|| format!("git {} failed", a.join(" ")))
                })
        };
        git(&[
            "config",
            "merge.shape.name",
            "Shape boards, element by element",
        ])?;
        git(&[
            "config",
            "merge.shape.driver",
            "shape merge %O %A %B --driver",
        ])?;
        let top = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| format!("git: {e}"))?;
        let top = String::from_utf8_lossy(&top.stdout).trim().to_string();
        let attrs = Path::new(&top).join(".gitattributes");
        let have = std::fs::read_to_string(&attrs).unwrap_or_default();
        // both extensions: `.diagram` boards from before the rename too
        let mut text = have.clone();
        for line in ["*.shape merge=shape", "*.diagram merge=shape"] {
            if !text.lines().any(|l| l.trim() == line) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(line);
                text.push('\n');
            }
        }
        // the old driver's line gives way to the new one
        let text: String = text
            .lines()
            .filter(|l| l.trim() != "*.diagram merge=diagram")
            .map(|l| format!("{l}\n"))
            .collect();
        if text != have {
            std::fs::write(&attrs, text).map_err(|e| format!("{}: {e}", attrs.display()))?;
        }
        eprintln!("git now merges .shape and .diagram files with `shape merge` (commit .gitattributes so others get it; each clone runs `shape merge --install` once)");
        return Ok(ExitCode::SUCCESS);
    }
    let [base, ours, theirs] = [0, 1, 2].map(|i| args.pos.get(i).cloned());
    let (Some(base), Some(ours), Some(theirs)) = (base, ours, theirs) else {
        return Err("usage: shape merge BASE OURS THEIRS [-o OUT] (or --install)".into());
    };
    let texts = [&base, &ours, &theirs].map(|f| read_text(f).unwrap_or_default());
    let docs: Vec<Option<Document>> = texts
        .iter()
        .map(|t| shape_core::dsl::parse(t).ok())
        .collect();
    let driver = args.flag("--driver");
    let (Some(b), Some(o), Some(t)) = (&docs[0], &docs[1], &docs[2]) else {
        // not all three read as boards: git's own line-by-line merge
        if driver {
            let st = Command::new("git")
                .args(["merge-file", &ours, &base, &theirs])
                .status()
                .map_err(|e| format!("git merge-file: {e}"))?;
            return Ok(if st.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        return Err("BASE, OURS and THEIRS have to be readable boards".into());
    };
    let m = shape_core::merge::merge3(b, o, t);
    let mut text = shape_core::patch::patch(&texts[1], o, &m.doc);
    if !m.conflicts.is_empty() {
        let what: Vec<String> = m.conflicts.iter().map(|c| c.key()).collect();
        text = format!(
            "# merge: {} conflict{} kept as yours ({}); open the board in the editor (`shape FILE`) to choose\n{text}",
            m.conflicts.len(),
            if m.conflicts.len() == 1 { "" } else { "s" },
            what.join(", ")
        );
        for c in &m.conflicts {
            eprintln!("conflict: {}", shape_core::merge::describe(c));
        }
    }
    let out = if driver {
        Some(ours.as_str())
    } else {
        args.val("--out")
    };
    match out {
        Some(p) if p != "-" => std::fs::write(p, &text).map_err(|e| format!("{p}: {e}"))?,
        _ => print!("{text}"),
    }
    Ok(if m.conflicts.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// `--canvas a/b`: commands read and change that canvas inside the board.
static CANVAS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

fn canvas_scope() -> &'static [String] {
    CANVAS.get().map(|v| v.as_slice()).unwrap_or(&[])
}

fn load(path: &str) -> Result<Document, String> {
    let doc = parse_doc(&read_text(path)?, path)?;
    let scope = canvas_scope();
    if scope.is_empty() {
        return Ok(doc);
    }
    doc.at(scope)
        .cloned()
        .ok_or_else(|| format!("{path} has no canvas `{}`", scope.join("/")))
}

/// A base version: another file, or the same file at a git ref.
fn load_base(spec: &str, file: &str) -> Result<Document, String> {
    if Path::new(spec).exists() {
        load(spec)
    } else {
        parse_doc(&git_show(spec, file)?, &format!("{file} at {spec}"))
    }
}

pub fn format_for(path: &str) -> &'static str {
    if path.ends_with(".json") {
        "json"
    } else if path.ends_with(".mmd") || path.ends_with(".mermaid") {
        "mermaid"
    } else {
        "dsl"
    }
}

fn save(path: &str, doc: &Document) -> Result<(), String> {
    // a canvas goes back into its board
    let whole;
    let doc = if canvas_scope().is_empty() || path == "-" {
        doc
    } else {
        let mut root = parse_doc(&read_text(path)?, path)?;
        *root
            .at_mut(canvas_scope())
            .ok_or_else(|| format!("{path} has no canvas `{}`", canvas_scope().join("/")))? =
            doc.clone();
        whole = root;
        &whole
    };
    let format = format_for(path);
    let mut text = shape_core::export(doc, format)?;
    if format == "dsl" && path != "-" {
        // keep the file as it is written: rewrite only what changed
        if let Ok(src) = std::fs::read_to_string(path) {
            if let Ok(old) = shape_core::dsl::parse(&src) {
                text = shape_core::patch::patch(&src, &old, doc);
            }
        }
    }
    if path == "-" {
        print!("{text}");
        return Ok(());
    }
    std::fs::write(path, text).map_err(|e| format!("{path}: {e}"))
}

fn write_out(out: Option<&str>, content: &[u8], what: &str) -> Result<(), String> {
    match out {
        Some(p) if p != "-" => {
            std::fs::write(p, content).map_err(|e| format!("{p}: {e}"))?;
            eprintln!("wrote {what} to {p} ({} KB)", content.len().div_ceil(1024));
            Ok(())
        }
        _ => {
            use std::io::Write;
            std::io::stdout()
                .write_all(content)
                .map_err(|e| e.to_string())
        }
    }
}

fn author(args: &Args) -> Option<String> {
    if let Some(b) = args.val("--by") {
        return Some(b.to_string());
    }
    let git = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());
    git.or_else(|| std::env::var("USER").ok())
}

/// `--today` (the date now, UTC) or `--as-of YYYY-MM-DD`: the day gantt
/// charts mark and forecast from (days since 1970).
fn today(args: &Args) -> Result<Option<i64>, String> {
    if let Some(d) = args.val("--as-of") {
        return shape_core::gantt::parse_date(d)
            .map(Some)
            .ok_or_else(|| format!("--as-of: `{d}` is not a date (YYYY-MM-DD)"));
    }
    if args.flag("--today") {
        return Ok(Some(today_utc()));
    }
    Ok(None)
}

/// A prompt with today's date on top, so the model does not guess the year.
fn dated(prompt: &str) -> String {
    format!(
        "Today is {}.\n\n{prompt}",
        shape_core::gantt::format_date(today_utc())
    )
}

/// Today (UTC), in days since 1970.
fn today_utc() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86400) as i64
}

fn render_opts(args: &Args) -> Result<RenderOptions, String> {
    let mut o = RenderOptions::default();
    if args.flag("--dark") {
        o.theme = Some(Theme::Dark);
    }
    if let Some(t) = args.val("--theme") {
        o.theme = Some(Theme::parse(t).ok_or_else(|| format!("unknown theme {t}"))?);
    }
    o.transparent = args.flag("--transparent");
    o.today = today(args)?.map(shape_core::gantt::format_date);
    if args.flag("--title") {
        o.title = Some(true);
    }
    if let Some(f) = args.val("--focus") {
        o.focus = f
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    o.comments = match args.val("--comments").unwrap_or("open") {
        "open" => CommentMode::Open,
        "all" => CommentMode::All,
        "none" => CommentMode::None,
        m => return Err(format!("--comments must be open, all or none (got {m})")),
    };
    Ok(o)
}

/// The theme file a board names (`theme: ./brand.theme`), read next to it.
pub fn theme_file(doc: &Document, file: &str) -> Result<Option<(String, String)>, String> {
    let Some(name) = doc.settings.palette.as_deref() else {
        return Ok(None);
    };
    if !shape_core::theme::is_file(name) || doc.themes.iter().any(|t| t.id == name) {
        return Ok(None);
    }
    let dir = Path::new(file).parent().unwrap_or(Path::new("."));
    let path = dir.join(name);
    let src =
        std::fs::read_to_string(&path).map_err(|e| format!("theme {}: {e}", path.display()))?;
    Ok(Some((name.to_string(), src)))
}

/// The board's theme from its file, as a render option.
fn file_themes(doc: &Document, file: &str) -> Result<Vec<shape_core::model::ThemeDef>, String> {
    Ok(match theme_file(doc, file)? {
        Some((name, src)) => vec![shape_core::theme::from_file(&name, &src)?],
        None => Vec::new(),
    })
}

fn rendered(args: &Args, file: &str) -> Result<(Document, shape_core::Rendered), String> {
    let doc = load(file)?;
    let mut opts = render_opts(args)?;
    opts.themes = file_themes(&doc, file)?;
    // a perspective's dashboard (`--base` is what it says changed since)
    if let Some(id) = args.val("--perspective") {
        let p = shape_core::perspectives::all(&doc)
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                let ids: Vec<String> = shape_core::perspectives::all(&doc).into_iter().map(|p| p.id).collect();
                format!("no perspective `{id}` (this board has: {})", ids.join(", "))
            })?;
        let now = today(args)?.unwrap_or_else(today_utc);
        let before = args.val("--base").map(|b| load_base(b, file)).transpose()?;
        let dash = shape_core::dashboard::build(&doc, &p, now, before.as_ref());
        let r = shape_core::render::render(&dash, &opts);
        return Ok((doc, r));
    }
    let r = match args.val("--base") {
        Some(b) => shape_core::render_diff(&load_base(b, file)?, &doc, &opts),
        None => shape_core::render::render(&doc, &opts),
    };
    Ok((doc, r))
}

fn default_out(file: &str, ext: &str) -> Option<String> {
    if file == "-" {
        return None;
    }
    let p = Path::new(file);
    let stem = p.file_name()?.to_str()?.split('.').next()?.to_string();
    Some(
        p.with_file_name(format!("{stem}.{ext}"))
            .to_string_lossy()
            .into_owned(),
    )
}

pub const TEMPLATES: &[(&str, &str)] = &[
    ("flow", include_str!("../templates/flow.shape")),
    (
        "architecture",
        include_str!("../templates/architecture.shape"),
    ),
    ("board", include_str!("../templates/board.shape")),
    // boards with a type, and so with perspectives
    ("product", include_str!("../templates/product.shape")),
    ("project", include_str!("../templates/project.shape")),
];

fn run(args: Args) -> Result<ExitCode, String> {
    let ok = Ok(ExitCode::SUCCESS);
    match args.cmd.as_str() {
        "help" => {
            print!("{HELP}");
            ok
        }
        "version" => {
            println!("shape {}", env!("CARGO_PKG_VERSION"));
            ok
        }
        "open" => {
            use window::{Native, Open};
            let port = args
                .val("--port")
                .map(|p| p.parse::<u16>().map_err(|_| "--port must be a number"))
                .transpose()?;
            let open = window::choose(
                args.flag("--no-open"),
                args.flag("--browser"),
                cfg!(feature = "window"),
            );
            // made first: on macOS the window owns the app before any dialog
            let native = match open {
                Open::Window => Native::init(),
                _ => None,
            };
            let file = match args.pos.first() {
                Some(f) => PathBuf::from(f),
                None => {
                    // from a terminal or a double-click on the app: ask where
                    // the new board goes (not from scripts and tests)
                    let terminal = std::io::stdin().is_terminal();
                    let asked = native
                        .as_ref()
                        .filter(|_| terminal || args.flag("--idle-exit"));
                    match asked {
                        Some(n) => match n.ask_new_board(&window::start_dir(terminal)) {
                            Some(f) => f,
                            None => return ok, // cancelled
                        },
                        None => PathBuf::from(untitled()),
                    }
                }
            };
            if !file.exists() {
                std::fs::write(&file, "idea: Start here\nidea -> next: then\n")
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                let at = std::fs::canonicalize(&file).unwrap_or_else(|_| file.clone());
                eprintln!("created {}", at.display());
            }
            let show = match (open, native) {
                (Open::Nowhere, _) => server::Show::None,
                (Open::Window, Some(n)) => server::Show::Window(n),
                (Open::Window, None) => {
                    if cfg!(feature = "window") {
                        eprintln!("shape: no window here (no display?); opening the browser");
                    }
                    server::Show::Browser
                }
                (Open::Browser, _) => server::Show::Browser,
            };
            let ai = ai::command(args.val("--ai"));
            server::serve(file, port, show, ai, args.flag("--idle-exit"))?;
            ok
        }
        "install-app" => {
            app::install(args.flag("--remove"))?;
            ok
        }
        "new" => {
            let file = args.file(0)?;
            if Path::new(file).exists() {
                return Err(format!("{file} already exists"));
            }
            let name = args.val("--template").unwrap_or("flow");
            let (_, body) = TEMPLATES.iter().find(|(n, _)| *n == name).ok_or_else(|| {
                format!("unknown template {name} (flow, architecture, board, product, project)")
            })?;
            let doc = parse_doc(body, name)?;
            save(file, &doc)?;
            eprintln!("created {file} from the {name} template — open it with `shape {file}`");
            ok
        }
        "svg" => {
            let file = args.file(0)?;
            let (_, r) = rendered(&args, file)?;
            write_out(args.val("-o"), r.svg.as_bytes(), "SVG")?;
            ok
        }
        "png" => {
            let file = args.file(0)?;
            let (_, r) = rendered(&args, file)?;
            let scale = args
                .val("--scale")
                .map(|s| s.parse::<f32>().map_err(|_| "--scale must be a number"))
                .transpose()?
                .unwrap_or(2.0);
            let png = png::render(&r.svg, scale)?;
            let out = args
                .val("-o")
                .map(String::from)
                .or_else(|| default_out(file, "png"));
            write_out(out.as_deref(), &png, "PNG")?;
            ok
        }
        "html" => {
            let file = args.file(0)?;
            let src = read_text(file)?;
            let mut doc = parse_doc(&src, file)?;
            // a theme from a file goes into the page with the board
            if let Some(mut t) = file_themes(&doc, file)?.pop() {
                let id = Path::new(&t.id)
                    .file_stem()
                    .map(|s| shape_core::model::slug(&s.to_string_lossy()))
                    .unwrap_or_else(|| "theme".into());
                t.id = id.clone();
                doc.themes.push(t);
                doc.settings.palette = Some(id);
            }
            let base = args.val("--base").map(|b| load_base(b, file)).transpose()?;
            let html = export::html(&doc, base.as_ref(), &render_opts(&args)?);
            let out = args
                .val("-o")
                .map(String::from)
                .or_else(|| default_out(file, "html"));
            write_out(out.as_deref(), html.as_bytes(), "HTML")?;
            ok
        }
        "convert" | "brief" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            let to = if args.cmd == "brief" {
                "brief"
            } else {
                args.val("--to")
                    .ok_or("--to dsl|json|mermaid|brief is required")?
            };
            let text = match today(&args)? {
                Some(t) if to == "brief" => shape_core::prompt::brief_at(&doc, Some(t)),
                _ => shape_core::export(&doc, to)?,
            };
            write_out(args.val("-o"), text.as_bytes(), to)?;
            ok
        }
        "schema" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let dir = Path::new(file)
                .parent()
                .filter(|d| !d.as_os_str().is_empty())
                .unwrap_or(Path::new("."))
                .to_path_buf();
            // (the canvas it is in, the block, its file's path from the board, the source)
            let mut todo: Vec<(Vec<String>, Option<String>, String, String)> = Vec::new();
            let blocks = schema_blocks(&doc, &[]);
            match args.pos.get(1) {
                Some(src) => {
                    let rel = relative_to(&dir, Path::new(src));
                    let found = match args.val("--into") {
                        Some(id) => blocks.iter().find(|(_, g, _)| g == id),
                        None => blocks.iter().find(|(_, _, f)| *f == rel),
                    };
                    let (at, into) = match found {
                        Some((at, g, _)) => (at.clone(), Some(g.clone())),
                        None => (Vec::new(), args.val("--into").map(String::from)),
                    };
                    todo.push((at, into, rel, read_text(src)?));
                }
                None => {
                    for (at, id, f) in blocks {
                        let text = read_text(&dir.join(&f).to_string_lossy())
                            .map_err(|e| format!("schema `{id}`: {e}"))?;
                        todo.push((at, Some(id), f, text));
                    }
                    if todo.is_empty() {
                        return Err(format!(
                            "{file} has no schema blocks with a file: add one with `shape schema {file} db/structure.sql`"
                        ));
                    }
                }
            }
            let before = doc.clone();
            for (at, into, rel, text) in todo {
                let here = doc.at(&at).ok_or("no such canvas")?.clone();
                let mut sync = match &into {
                    Some(id) => {
                        let mut with = here.clone();
                        let mut first = Vec::new();
                        if let Some(o) = args.val("--only") {
                            first.push(serde_json::json!({"op": "update_group", "id": id, "only": o}));
                            apply_json(&mut with, &first)?;
                        }
                        let s = shape_core::schemas::sync_ops(&with, id, &text)?;
                        shape_core::schemas::Sync { ops: [first, s.ops].concat(), ..s }
                    }
                    None => shape_core::schemas::add_ops(
                        &here,
                        args.val("--label").unwrap_or("Database"),
                        Some(&rel),
                        args.val("--only"),
                        &text,
                    )?,
                };
                for w in &sync.warnings {
                    eprintln!("{rel}: {w}");
                }
                eprintln!("{rel}: {}", sync.summary());
                if !at.is_empty() {
                    sync.ops = vec![serde_json::json!({"op": "in", "canvas": at.join("/"), "ops": sync.ops})];
                }
                apply_json(&mut doc, &sync.ops)?;
            }
            if args.flag("--dry-run") {
                let d = shape_core::diff::diff(&before, &doc);
                print!("{}", shape_core::diff::markdown(&d, &before, &doc));
            } else if before != doc {
                save(file, &doc)?;
            }
            ok
        }
        "schedule" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            let now = match today(&args)? {
                Some(t) => t,
                None => today_utc(),
            };
            let charts: Vec<_> = doc
                .groups
                .iter()
                .filter(|g| g.gantt && args.pos.get(1).is_none_or(|c| *c == g.id))
                .collect();
            if charts.is_empty() {
                return Err(match args.pos.get(1) {
                    Some(c) => format!("no gantt chart `{c}`"),
                    None => "no gantt charts".into(),
                });
            }
            let all: Vec<_> = charts
                .iter()
                .map(|g| (g, shape_core::gantt::track(&doc, &g.id, now)))
                .collect();
            if args.flag("--json") {
                let v: Vec<_> = all
                    .iter()
                    .map(|(g, t)| serde_json::json!({"id": g.id, "label": g.display(), "tasks": t}))
                    .collect();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "today": shape_core::gantt::format_date(now),
                        "charts": v
                    }))
                    .unwrap()
                );
                return ok;
            }
            for (g, tasks) in all {
                println!(
                    "{} ({}), as of {}",
                    g.display(),
                    g.id,
                    shape_core::gantt::format_date(now)
                );
                let w = tasks
                    .iter()
                    .filter_map(|t| doc.node(&t.id))
                    .map(|n| n.display().chars().count())
                    .max()
                    .unwrap_or(4)
                    .min(36);
                for t in tasks {
                    let name: String = doc
                        .node(&t.id)
                        .map_or(t.id.clone(), |n| n.display().chars().take(w).collect());
                    let when = if t.milestone || t.start == t.end {
                        t.start.clone()
                    } else {
                        format!("{} → {}", t.start, t.end)
                    };
                    let done = t.done.map(|d| format!("{d}%")).unwrap_or_default();
                    let fc = if t.slip > 0 {
                        format!("forecast {} (+{}d)", t.forecast_end, t.slip)
                    } else {
                        String::new()
                    };
                    println!(
                        "  {name:<w$}  {when:<23}  {done:>4}  {:<11}  {fc}",
                        t.status
                    );
                }
                println!();
            }
            ok
        }
        "comments" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            let list: Vec<_> = doc
                .comments
                .iter()
                .filter(|c| args.flag("--all") || c.status == Status::Open)
                .collect();
            if args.flag("--json") {
                println!("{}", serde_json::to_string_pretty(&list).unwrap());
                return ok;
            }
            if list.is_empty() {
                eprintln!(
                    "no {}comments",
                    if args.flag("--all") { "" } else { "open " }
                );
            }
            for c in list {
                let on =
                    c.on.as_deref()
                        .map(|o| doc.describe(o))
                        .unwrap_or_else(|| "canvas".into());
                let by = c
                    .author
                    .as_deref()
                    .map(|a| format!(" @{a}"))
                    .unwrap_or_default();
                let st = if c.status == Status::Resolved {
                    " [resolved]"
                } else {
                    ""
                };
                println!(
                    "{} on {on}{by}{st}\n    {}",
                    c.id,
                    c.text.replace('\n', "\n    ")
                );
                for r in &c.replies {
                    println!(
                        "    ↳ {}{}",
                        r.author
                            .as_deref()
                            .map(|a| format!("@{a}: "))
                            .unwrap_or_default(),
                        r.text
                    );
                }
                if let Some(res) = &c.resolution {
                    println!("    ✓ {res}");
                }
            }
            ok
        }
        "comment" | "reply" | "resolve" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let op = match args.cmd.as_str() {
                "comment" => {
                    let text = args.pos[1..].join(" ");
                    if text.trim().is_empty() {
                        return Err("usage: shape comment FILE [--on ID] TEXT".into());
                    }
                    Op::AddComment {
                        id: None,
                        on: args.val("--on").map(String::from),
                        text,
                        author: author(&args),
                        x: None,
                        y: None,
                    }
                }
                "reply" => {
                    let id = args
                        .pos
                        .get(1)
                        .ok_or("usage: shape reply FILE COMMENT_ID TEXT")?
                        .clone();
                    let text = args.pos[2..].join(" ");
                    Op::Reply {
                        id,
                        text,
                        author: author(&args),
                    }
                }
                _ => {
                    let id = args
                        .pos
                        .get(1)
                        .ok_or("usage: shape resolve FILE COMMENT_ID [--note TEXT]")?
                        .clone();
                    Op::ResolveComment {
                        id,
                        resolution: args.val("--note").map(String::from),
                    }
                }
            };
            let r = shape_core::ops::apply(&mut doc, &[op]).map_err(|e| e.message)?;
            save(file, &doc)?;
            if let Some(id) = r.created.first() {
                println!("{id}");
            }
            ok
        }
        "prompt" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            // the whole board (canvases too) as a prompt to build it
            if args.val("--target") == Some("build") {
                print!(
                    "{}",
                    shape_core::prompt::build(&doc, today(&args)?.or(Some(today_utc())))
                );
                return Ok(ExitCode::SUCCESS);
            }
            let target = match args.val("--target").unwrap_or("diagram") {
                "diagram" => Target::Diagram,
                "code" => Target::Code,
                "discuss" => Target::Discuss,
                t => {
                    return Err(format!(
                        "--target must be build, diagram, code or discuss (got {t})"
                    ))
                }
            };
            let reply = match args.val("--reply").unwrap_or("ops") {
                "ops" => Reply::Ops,
                "dsl" => Reply::Dsl,
                r => return Err(format!("--reply must be ops or dsl (got {r})")),
            };
            let comments: Vec<String> = args
                .val("--comment")
                .map(|c| {
                    c.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            for c in &comments {
                if doc.comment(c).is_none() {
                    return Err(format!("no comment {c}"));
                }
            }
            let opts = PromptOptions {
                target,
                reply,
                comments,
                include_resolved: args.flag("--all"),
                instructions: args.val("--instructions").map(String::from),
                file: (file != "-").then(|| file.to_string()),
                context: match args.val("--context").unwrap_or("auto") {
                    "auto" => shape_core::prompt::Context::Auto,
                    "near" => shape_core::prompt::Context::Near,
                    "section" => shape_core::prompt::Context::Section,
                    "all" => shape_core::prompt::Context::All,
                    c => {
                        return Err(format!(
                            "--context must be auto, near, section or all (got {c})"
                        ))
                    }
                },
            };
            print!("{}", shape_core::prompt::prompt(&doc, &opts));
            ok
        }
        "outline" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            let id = match args.pos.get(1) {
                Some(id) => id.clone(),
                None => first_map(&doc)?,
            };
            if doc.node(&id).is_none() {
                return Err(format!("no node `{id}`"));
            }
            if args.flag("--md") {
                print!(
                    "{}",
                    shape_core::prompt::selection_at(&doc, &[id], today(&args)?)
                );
            } else {
                print!("{}", shape_core::mindmap::outline(&doc, &id, true));
            }
            ok
        }
        "ask" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let request = args.pos[1..].join(" ");
            if request.trim().is_empty() {
                return Err("usage: shape ask FILE \"what to change\" [--on ID,ID] [--dry-run] [--print] [--ai claude|codex|gemini|CMD]".into());
            }
            let ids: Vec<String> = args
                .val("--on")
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let mut prompt = shape_core::prompt::ask(&doc, &ids, &request, &team_recipes(&file));
            let cmd = match ai::command(args.val("--ai")) {
                Some(c) if !args.flag("--print") => c,
                _ => {
                    // no harness: the prompt, for any chat; `shape apply` takes the reply
                    print!("{prompt}");
                    return ok;
                }
            };
            let before = doc.clone();
            // a reply that does not apply goes back with its error, twice at most
            let mut tries = 0;
            let r = loop {
                eprintln!("asking `{cmd}`…");
                let reply = ai::run(&cmd, &dated(&prompt))?;
                let applied = shape_core::ops::parse_ops(&reply)
                    .map_err(|e| e.to_string())
                    .and_then(|ops| {
                        let mut d = before.clone();
                        shape_core::ops::apply(&mut d, &ops)
                            .map(|r| (d, r))
                            .map_err(|e| e.to_string())
                    });
                match applied {
                    Ok((d, r)) => {
                        doc = d;
                        break r;
                    }
                    Err(e) if tries < 2 => {
                        tries += 1;
                        eprintln!("the reply did not apply ({e}); asking again");
                        prompt = shape_core::prompt::ask_again(&prompt, &reply, &e);
                    }
                    Err(e) => return Err(format!("the model's reply did not apply: {e}")),
                }
            };
            print!(
                "{}",
                shape_core::diff::markdown(&shape_core::diff::diff(&before, &doc), &before, &doc)
            );
            if args.flag("--dry-run") {
                eprintln!("dry run: {} ops ok, {file} not changed", r.count);
            } else {
                save(file, &doc)?;
                eprintln!("applied {} ops to {file}", r.count);
            }
            ok
        }
        "expand" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let o = shape_core::mindmap::ExpandOptions {
                instructions: args.val("--ask").map(String::from),
                count: args.val("--count").and_then(|c| c.parse().ok()),
                levels: args.val("--levels").and_then(|c| c.parse().ok()),
            };
            let (prompt, parent) = match args.val("--new") {
                Some(topic) => (shape_core::mindmap::generate_prompt(topic, &o), None),
                None => {
                    let id = args
                        .pos
                        .get(1)
                        .ok_or("usage: shape expand FILE ID (or --new TOPIC)")?;
                    (
                        shape_core::mindmap::expand_prompt(&doc, id, &o)?,
                        Some(id.clone()),
                    )
                }
            };
            let cmd = ai::command(args.val("--ai"));
            let Some(cmd) = cmd.filter(|_| !args.flag("--print")) else {
                print!("{prompt}");
                if !args.flag("--print") {
                    eprintln!(
                        "no AI harness found: pass --ai CMD or set SHAPE_AI; the prompt is above"
                    );
                }
                return ok;
            };
            eprintln!("asking `{cmd}`…");
            let reply = ai::run(&cmd, &dated(&prompt))?;
            let outline: String = shape_core::mindmap::parse_reply(&reply)
                .iter()
                .map(|l| format!("{}{}\n", "  ".repeat(l.depth), l.text))
                .collect();
            let r = shape_core::ops::apply(
                &mut doc,
                &[Op::AddOutline {
                    parent,
                    outline: outline.clone(),
                    index: None,
                    x: None,
                    y: None,
                    group: args.val("--in").map(str::to_string),
                }],
            )
            .map_err(|e| e.to_string())?;
            save(file, &doc)?;
            print!("{outline}");
            eprintln!("added {} topics to {file}", r.created.len());
            ok
        }
        "apply" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let patch = match args.pos.get(1).map(|s| s.as_str()).unwrap_or("-") {
                // ops right on the command line
                inline if inline.trim_start().starts_with(['{', '[']) => inline.to_string(),
                path => read_text(path)?,
            };
            // a reply may also be a whole new board in a ```shape block
            // (```diagram, its name before the rename, too)
            let fenced = ["```shape", "```diagram"]
                .iter()
                .find_map(|f| patch.find(f).map(|i| i + f.len()));
            let ops = match shape_core::ops::parse_ops(&patch) {
                Ok(ops) => ops,
                Err(e) => match fenced {
                    Some(start) => {
                        let body = &patch[start..];
                        let body = &body[..body.find("```").unwrap_or(body.len())];
                        let mut new = parse_doc(body, "the ```shape block")?;
                        // prompts leave the images' data out: keep the board's
                        new.adopt_hidden(&doc);
                        if args.flag("--dry-run") {
                            print!(
                                "{}",
                                shape_core::diff::markdown(
                                    &shape_core::diff::diff(&doc, &new),
                                    &doc,
                                    &new
                                )
                            );
                        } else {
                            save(file, &new)?;
                            eprintln!("replaced {file} with the diagram from the reply");
                        }
                        return ok;
                    }
                    None => return Err(e),
                },
            };
            let before = doc.clone();
            let r = shape_core::ops::apply(&mut doc, &ops).map_err(|e| e.to_string())?;
            let d = shape_core::diff::diff(&before, &doc);
            print!("{}", shape_core::diff::markdown(&d, &before, &doc));
            if args.flag("--dry-run") {
                eprintln!("dry run: {} ops ok, {file} not changed", r.count);
            } else {
                save(file, &doc)?;
                eprintln!("applied {} ops to {file}", r.count);
            }
            ok
        }
        "query" => {
            let file = args.file(0)?;
            let doc = load(file)?;
            let sels: Vec<&str> = args.pos[1..].iter().map(|s| s.as_str()).collect();
            if sels.is_empty() {
                return Err("usage: shape query FILE SELECTOR... (see `shape help`)".into());
            }
            if args.flag("--ids") {
                for id in shape_core::query::select(&doc, &sels)? {
                    println!("{id}");
                }
            } else {
                print!("{}", shape_core::query::query(&doc, &sels)?);
            }
            ok
        }
        "add" => {
            let file = args.file(0)?;
            let mut doc = load(file)?;
            let dsl = match args.pos.get(1).map(|s| s.as_str()).unwrap_or("-") {
                "-" => read_text("-")?,
                text => text.to_string(),
            };
            let before = doc.clone();
            let r =
                shape_core::ops::apply(&mut doc, &[Op::Merge { dsl }]).map_err(|e| e.message)?;
            print!(
                "{}",
                shape_core::diff::markdown(&shape_core::diff::diff(&before, &doc), &before, &doc)
            );
            if args.flag("--dry-run") {
                eprintln!("dry run: {file} not changed");
            } else {
                save(file, &doc)?;
                eprintln!("added to {file} ({} new)", r.created.len());
            }
            ok
        }
        "check" => {
            let mut failed = false;
            if args.pos.is_empty() {
                return Err("usage: shape check FILE...".into());
            }
            for f in &args.pos {
                let src = read_text(f)?;
                match shape_core::parse(&src) {
                    Ok(doc) => {
                        let mut problems = doc.check();
                        problems.extend(
                            board_link_problems(&doc, f)
                                .into_iter()
                                .map(|m| shape_core::model::Diagnostic::new(0, m)),
                        );
                        if problems.is_empty() {
                            let open = doc
                                .comments
                                .iter()
                                .filter(|c| c.status == Status::Open)
                                .count();
                            eprintln!(
                                "{f}: ok ({} nodes, {} edges, {open} open comments)",
                                doc.nodes.len(),
                                doc.edges.len()
                            );
                        } else {
                            failed = true;
                            for p in problems {
                                eprintln!("{f}: {p}");
                            }
                        }
                    }
                    Err(errs) => {
                        failed = true;
                        for e in errs {
                            eprintln!("{f}:{}: {}", e.line, e.message);
                        }
                    }
                }
            }
            Ok(if failed {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        "fmt" => {
            let mut dirty = false;
            for f in &args.pos {
                let src = read_text(f)?;
                let doc = parse_doc(&src, f)?;
                let out = shape_core::export(&doc, format_for(f))?;
                if out != src {
                    dirty = true;
                    if args.flag("--check") {
                        eprintln!("{f} is not formatted");
                    } else if f == "-" {
                        print!("{out}");
                    } else {
                        std::fs::write(f, out).map_err(|e| e.to_string())?;
                        eprintln!("formatted {f}");
                    }
                } else if f == "-" {
                    print!("{out}");
                }
            }
            Ok(if dirty && args.flag("--check") {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        "merge" => merge_cmd(&args),
        "diff" => {
            let a = args.file(0)?;
            let b = args
                .pos
                .get(1)
                .ok_or("usage: shape diff OLD NEW  or  shape diff FILE GIT_REF")?;
            // `shape diff FILE REF` compares the file at REF (old) with the working copy (new)
            let (old, new) = if Path::new(b).exists() {
                (load(a)?, load(b)?)
            } else {
                (load_base(b, a)?, load(a)?)
            };
            let d = shape_core::diff::diff(&old, &new);
            if args.flag("--json") {
                println!("{}", serde_json::to_string_pretty(&d).unwrap());
            } else {
                print!("{}", shape_core::diff::markdown(&d, &old, &new));
            }
            ok
        }
        "spec" => {
            if args.flag("--json-schema") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&shape_core::json_schema()).unwrap()
                );
            } else {
                print!("{}", shape_core::spec());
            }
            ok
        }
        "embed" => {
            let out = args.val("-o").unwrap_or("shape.embed.js");
            write_out(
                Some(out),
                export::embed_bundle().as_bytes(),
                "the embed bundle",
            )?;
            ok
        }
        "skills" => {
            skills::run(
                &args.pos,
                args.flag("--global"),
                args.val("--dir").map(PathBuf::from),
            )?;
            ok
        }
        c => Err(format!("unknown command {c}")),
    }
}

/// Apply ops written as JSON values.
fn apply_json(doc: &mut Document, ops: &[serde_json::Value]) -> Result<(), String> {
    let ops = shape_core::ops::parse_ops(&serde_json::json!({ "ops": ops }).to_string())?;
    shape_core::ops::apply(doc, &ops)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// `path` as seen from directory `dir` (`db/structure.sql`, `../api/db/schema.rb`),
/// with `/` between the parts.
pub(crate) fn relative_to(dir: &Path, path: &Path) -> String {
    let abs = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (from, to) = (abs(dir), abs(path));
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(b[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

/// The schema blocks with a file, in the board and the canvases inside it:
/// (the canvas's path, the block's id, its file).
fn schema_blocks(doc: &Document, at: &[String]) -> Vec<(Vec<String>, String, String)> {
    let mut out: Vec<_> = doc
        .groups
        .iter()
        .filter(|g| g.schema)
        .filter_map(|g| Some((at.to_vec(), g.id.clone(), g.file.clone()?)))
        .collect();
    for c in &doc.canvases {
        let path = [at, &[c.id.clone()]].concat();
        out.extend(schema_blocks(&c.doc, &path));
    }
    out
}
