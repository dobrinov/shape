//! Session logs: what a Shape instance did, one JSON object per line, so
//! the owner's feedback can be checked against what really happened (`shape
//! logs`). One file per session in the platform's log folder; the last
//! [`KEEP`] are kept. Local only. `SHAPE_NO_LOG=1` turns them off,
//! `SHAPE_LOG_DIR` puts them somewhere else.
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

/// Sessions kept.
const KEEP: usize = 30;

static LOG: Mutex<Option<File>> = Mutex::new(None);
static DEBUG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// What only `--debug` logs: every op, each perspective and canvas opened,
/// the editor's warnings.
const VERBOSE: &[&str] = &["op", "perspective", "canvas", "warn"];

/// `--debug`: log the details too.
pub fn set_debug(on: bool) {
    DEBUG.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn debug() -> bool {
    DEBUG.load(std::sync::atomic::Ordering::Relaxed)
}

fn env(k: &str) -> Option<String> {
    std::env::var(k)
        .ok()
        .or_else(|| std::env::var(k.replacen("SHAPE_", "DIAGRAM_", 1)).ok())
        .filter(|v| !v.is_empty())
}

/// Where the logs go.
pub fn dir() -> Option<PathBuf> {
    if let Some(d) = env("SHAPE_LOG_DIR") {
        return Some(PathBuf::from(d));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Logs/Shape"))
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("Shape").join("logs"))
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/state")))
            .map(|d| d.join("shape/logs"))
    }
}

fn off() -> bool {
    env("SHAPE_NO_LOG").is_some_and(|v| v != "0")
}

/// UTC time, to the second: `2026-10-21T08:34:03Z`.
pub fn stamp(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rest = secs % 86_400;
    // days to a civil date (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Starts this session's log (a no-op when logs are off or cannot be written).
pub fn start(info: Value) {
    if off() {
        return;
    }
    let Some(dir) = dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    prune(&dir);
    let name = format!(
        "{}-{}.jsonl",
        stamp(now()).replace(':', "-").trim_end_matches('Z'),
        std::process::id()
    );
    let Ok(f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
    else {
        return;
    };
    *LOG.lock().unwrap_or_else(|e| e.into_inner()) = Some(f);
    event("start", info);
}

fn prune(dir: &PathBuf) {
    let mut files = sessions_in(dir);
    while files.len() >= KEEP {
        let _ = std::fs::remove_file(files.remove(0));
    }
}

fn sessions_in(dir: &PathBuf) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    files.sort();
    files
}

/// Adds an event: `{"t": …, "kind": KIND, …fields}`.
pub fn event(kind: &str, fields: Value) {
    if VERBOSE.contains(&kind) && !debug() {
        return;
    }
    let mut guard = LOG.lock().unwrap_or_else(|e| e.into_inner());
    let Some(f) = guard.as_mut() else { return };
    let mut line = json!({ "t": stamp(now()), "kind": kind });
    if let (Some(o), Value::Object(more)) = (line.as_object_mut(), fields) {
        o.extend(more);
    }
    let _ = writeln!(f, "{line}");
}

/// `shape logs`: a session as text, for reading (or `--json` as written).
pub fn show(
    filter: Option<&str>,
    flag: impl Fn(&str) -> bool,
    val: impl Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let (just_ai, just_errors) = match filter {
        None => (false, false),
        Some("ai") => (true, false),
        Some("errors") => (false, true),
        Some(x) => return Err(format!("`{x}`: shape logs [ai|errors] (see shape help)")),
    };
    let dir =dir().ok_or("no log folder (HOME is not set)")?;
    let files = sessions_in(&dir);
    if files.is_empty() {
        return Err(format!(
            "no sessions logged yet in {}{}",
            dir.display(),
            if off() { " (SHAPE_NO_LOG is set)" } else { "" }
        ));
    }
    if flag("--clear") {
        for f in &files {
            let _ = std::fs::remove_file(f);
        }
        eprintln!(
            "removed {} session logs from {}",
            files.len(),
            dir.display()
        );
        return Ok(());
    }
    if flag("--list") {
        for (i, f) in files.iter().rev().enumerate() {
            let text = std::fs::read_to_string(f).unwrap_or_default();
            let first: Value = text
                .lines()
                .next()
                .and_then(|l| serde_json::from_str(l).ok())
                .unwrap_or(Value::Null);
            let ai = text
                .lines()
                .filter(|l| l.contains(r#""kind":"ai""#))
                .count();
            let errors = text
                .lines()
                .filter(|l| l.contains(r#""kind":"error""#) || l.contains(r#""kind":"op-error""#))
                .count();
            println!(
                "{:>2}  {}  {}  ({} lines, {ai} AI requests, {errors} errors)",
                i + 1,
                first["t"].as_str().unwrap_or("?"),
                first["file"].as_str().unwrap_or("?"),
                text.lines().count()
            );
        }
        println!(
            "\n{}\nshape logs --session N shows one (1 is the latest)",
            dir.display()
        );
        return Ok(());
    }
    let pick = val("--session").unwrap_or_else(|| "1".into());
    let file = match pick.parse::<usize>() {
        Ok(n) if n >= 1 && n <= files.len() => files[files.len() - n].clone(),
        _ => files
            .iter()
            .find(|f| f.to_string_lossy().contains(&pick))
            .cloned()
            .ok_or_else(|| format!("no session `{pick}` (see shape logs --list)"))?,
    };
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let only = |k: &str| {
        (!just_ai || matches!(k, "ai" | "ai-action" | "ai-result"))
            && (!just_errors || matches!(k, "error" | "op-error"))
    };
    let full = flag("--full");
    let cut = |s: &str, n: usize| -> String {
        if full || s.chars().count() <= n {
            s.to_string()
        } else {
            format!(
                "{}… ({} chars; --full for all)",
                s.chars().take(n).collect::<String>(),
                s.chars().count()
            )
        }
    };
    if flag("--json") {
        for l in text.lines() {
            let v: Value = serde_json::from_str(l).unwrap_or(Value::Null);
            if only(v["kind"].as_str().unwrap_or("")) {
                println!("{l}");
            }
        }
        return Ok(());
    }
    println!("{}\n", file.display());
    for l in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(l) else {
            continue;
        };
        let kind = v["kind"].as_str().unwrap_or("");
        if !only(kind) {
            continue;
        }
        let t = v["t"].as_str().unwrap_or("").get(11..19).unwrap_or("");
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        match kind {
            "start" => println!(
                "{t}  started Shape {} on {} ({}; AI: {})",
                s("version"),
                s("file"),
                s("url"),
                v["ai"].as_str().unwrap_or("none")
            ),
            "ai" => {
                println!(
                    "{t}  AI {} in {:.1}s via `{}`{}",
                    if v["ok"] == true {
                        "answered"
                    } else {
                        "failed"
                    },
                    v["ms"].as_f64().unwrap_or(0.0) / 1000.0,
                    s("command"),
                    match v["images"].as_u64() {
                        Some(n) if n > 0 => format!(" ({n} images)"),
                        _ => String::new(),
                    }
                );
                println!(
                    "      prompt: {}",
                    cut(&s("prompt"), 600).replace('\n', "\n              ")
                );
                if v["ok"] == true {
                    println!(
                        "      reply:  {}",
                        cut(&s("reply"), 600).replace('\n', "\n              ")
                    );
                } else {
                    println!("      error:  {}", s("error"));
                }
            }
            "save" => {
                println!(
                    "{t}  saved ({} bytes){}",
                    v["bytes"],
                    if s("by").is_empty() {
                        String::new()
                    } else {
                        format!(" by {}", s("by"))
                    }
                );
                if !s("changes").is_empty() {
                    println!(
                        "      {}",
                        cut(&s("changes"), 800).trim_end().replace('\n', "\n      ")
                    );
                }
            }
            "stop" => println!("{t}  stopped ({})", s("why")),
            _ => {
                // what the editor reported
                let mut rest = v.clone();
                if let Some(o) = rest.as_object_mut() {
                    o.remove("t");
                    o.remove("kind");
                }
                println!("{t}  {kind}: {}", cut(&rest.to_string(), 400));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn stamps_are_utc() {
        assert_eq!(super::stamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::stamp(1_792_571_643), "2026-10-21T08:34:03Z");
    }
}
