//! What Shape remembers for this user: where new boards are saved, and the
//! drafts of boards not saved anywhere yet.
//!
//! A new board (`shape` with no file) starts as a draft in Shape's own
//! folder, so nothing is lost while it has no name; the editor asks where it
//! goes once there is something on it. The folder people choose for new
//! boards, when they ask Shape to keep it, is in `prefs.json`.

use std::path::{Path, PathBuf};

/// Shape's own folder: `~/Library/Application Support/Shape` on macOS,
/// `%APPDATA%\Shape` on Windows, `$XDG_CONFIG_HOME/shape` elsewhere
/// (`SHAPE_CONFIG_DIR` puts it somewhere else).
pub fn dir() -> Option<PathBuf> {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if let Some(d) = env("SHAPE_CONFIG_DIR") {
        return Some(d);
    }
    if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/Shape"))
    } else if cfg!(windows) {
        env("APPDATA").map(|d| d.join("Shape"))
    } else {
        env("XDG_CONFIG_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".config")))
            .map(|d| d.join("shape"))
    }
}

fn file() -> Option<PathBuf> {
    dir().map(|d| d.join("prefs.json"))
}

fn read() -> serde_json::Map<String, serde_json::Value> {
    file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// The folder new boards are saved in, when someone chose to always use one.
pub fn new_board_dir() -> Option<PathBuf> {
    read()
        .get("new_boards")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .filter(|d| d.is_dir())
}

/// Always save new boards in `dir` (None: ask each time).
pub fn set_new_board_dir(dir: Option<&Path>) -> Result<(), String> {
    let f = file().ok_or("no folder for Shape's settings")?;
    let mut prefs = read();
    match dir {
        Some(d) => prefs.insert("new_boards".into(), d.display().to_string().into()),
        None => prefs.remove("new_boards"),
    };
    if let Some(p) = f.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(prefs)).unwrap_or_default();
    std::fs::write(&f, text + "\n").map_err(|e| format!("{}: {e}", f.display()))
}

/// What a new board starts with.
pub const STARTER: &str = "idea: Start here\nidea -> next: then\n";

/// Where drafts are kept.
pub fn drafts() -> Option<PathBuf> {
    dir().map(|d| d.join("drafts"))
}

/// Whether `file` is a draft (a board not saved anywhere yet).
pub fn is_draft(file: &Path) -> bool {
    let (Some(d), Ok(f)) = (drafts(), std::fs::canonicalize(file)) else {
        return false;
    };
    std::fs::canonicalize(d).is_ok_and(|d| f.starts_with(d))
}

/// The draft for a new board: the latest one someone started and left
/// unsaved (so closing Shape by accident loses nothing), else a new one.
pub fn draft() -> Result<PathBuf, String> {
    let d = drafts().ok_or("no folder for drafts")?;
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    let mut left: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&d)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "shape"))
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|s| s != STARTER))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    left.sort();
    if let Some((_, p)) = left.pop() {
        return Ok(p);
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|t| t.as_secs())
        .unwrap_or(0);
    let f = d.join(format!("Untitled-{secs}.shape"));
    std::fs::write(&f, STARTER).map_err(|e| format!("{}: {e}", f.display()))?;
    Ok(f)
}

/// A file name for a board called `name`: `Launch plan` → `Launch plan.shape`.
pub fn board_file_name(name: &str) -> Option<String> {
    let name = name.trim().trim_end_matches(".shape").trim();
    let clean: String = name
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let clean = clean.trim().trim_start_matches('.').to_string();
    (!clean.is_empty()).then(|| format!("{clean}.shape"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_the_folder_for_new_boards() {
        assert_eq!(
            board_file_name("Launch plan").as_deref(),
            Some("Launch plan.shape")
        );
        assert_eq!(
            board_file_name(" q3/plan.shape ").as_deref(),
            Some("q3plan.shape")
        );
        assert_eq!(board_file_name("  "), None);
        assert_eq!(board_file_name("..."), None);
    }
}
