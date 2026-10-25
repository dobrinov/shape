//! A board's history. In git it is the commits that touched the file; a
//! board git does not track gets snapshots instead, kept beside it in
//! `.shape-history/<file name>/` (one at most every few minutes of
//! saving, signed by whoever saved), so it can go back too.
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The least time between two snapshots of a file.
const EVERY: u64 = 5 * 60;
/// The most snapshots kept for a file (the oldest go first).
const KEEP: usize = 200;

/// One version of a board.
#[derive(Debug, Clone)]
pub struct Version {
    /// A commit hash, or a snapshot's file name.
    pub id: String,
    pub author: String,
    /// Seconds since 1970.
    pub time: u64,
    /// The commit message ("" for a snapshot).
    pub message: String,
}

fn git_in(dir: &Path, args: &[&str]) -> Option<String> {
    let out = crate::command("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The file's path in its git repository, when git tracks it.
fn tracked(file: &Path) -> Option<(PathBuf, String)> {
    let abs = std::fs::canonicalize(file).ok()?;
    let dir = abs.parent()?.to_path_buf();
    let rel = git_in(
        &dir,
        &[
            "ls-files",
            "--full-name",
            "--error-unmatch",
            &abs.to_string_lossy(),
        ],
    )?;
    let rel = rel.trim().to_string();
    (!rel.is_empty()).then_some((dir, rel))
}

/// Whether the file's history is git's.
pub fn in_git(file: &Path) -> bool {
    tracked(file).is_some()
}

fn snapshots_dir(file: &Path) -> Option<PathBuf> {
    let abs = std::fs::canonicalize(file).ok()?;
    let parent = abs.parent()?;
    // `.diagram-history` from before the rename to Shape stays in use
    let old = parent.join(".diagram-history").join(abs.file_name()?);
    if old.is_dir() {
        return Some(old);
    }
    Some(parent.join(".shape-history").join(abs.file_name()?))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A name as it goes in a snapshot's file name (and back).
fn sign(author: &str) -> String {
    let s: String = author
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    s.trim().replace(' ', "+")
}

impl Version {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "id": self.id, "author": self.author, "time": self.time, "message": self.message })
    }
}

/// The board's versions, newest first.
pub fn versions(file: &Path) -> Vec<Version> {
    if let Some((dir, rel)) = tracked(file) {
        let log = git_in(
            &dir,
            &[
                "log",
                "--follow",
                "-n",
                "100",
                "--format=%H%x1f%an%x1f%at%x1f%s",
                "--",
                &rel,
            ],
        )
        .unwrap_or_default();
        return log
            .lines()
            .filter_map(|l| {
                let mut p = l.split('\u{1f}');
                Some(Version {
                    id: p.next()?.to_string(),
                    author: p.next()?.to_string(),
                    time: p.next()?.parse().ok()?,
                    message: p.next().unwrap_or("").to_string(),
                })
            })
            .collect();
    }
    let Some(dir) = snapshots_dir(file) else {
        return Vec::new();
    };
    let mut out: Vec<Version> = std::fs::read_dir(&dir)
        .map(|r| {
            r.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|name| {
            let stem = name
                .strip_suffix(".shape")
                .or_else(|| name.strip_suffix(".diagram"))?;
            let (time, who) = stem.split_once("__").unwrap_or((stem, ""));
            Some(Version {
                time: time.parse().ok()?,
                author: who.replace('+', " "),
                message: String::new(),
                id: name.clone(),
            })
        })
        .collect();
    out.sort_by(|a, b| b.time.cmp(&a.time));
    out
}

/// The board as it was at version `id`.
pub fn version(file: &Path, id: &str) -> Result<String, String> {
    if let Some((dir, rel)) = tracked(file) {
        if !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("`{id}` is not a commit"));
        }
        return git_in(&dir, &["show", &format!("{id}:{rel}")])
            .ok_or_else(|| format!("the board is not in commit {id}"));
    }
    if id.contains('/') || id.contains('\\') || id.starts_with('.') {
        return Err(format!("`{id}` is not a version"));
    }
    let dir = snapshots_dir(file).ok_or("no history")?;
    std::fs::read_to_string(dir.join(id)).map_err(|e| format!("{id}: {e}"))
}

/// After a save: keep a snapshot of a board git does not track, at most one
/// every few minutes. Returns whether one was made.
pub fn snapshot(file: &Path, text: &str, author: &str) -> bool {
    if in_git(file) {
        return false;
    }
    let Some(dir) = snapshots_dir(file) else {
        return false;
    };
    let list = versions(file);
    let t = now();
    if list
        .first()
        .is_some_and(|v| t.saturating_sub(v.time) < EVERY)
    {
        return false;
    }
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let name = format!("{t}__{}.shape", sign(author));
    if std::fs::write(dir.join(&name), text).is_err() {
        return false;
    }
    for old in list.iter().skip(KEEP - 1) {
        let _ = std::fs::remove_file(dir.join(&old.id));
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_beside_a_board_git_does_not_track() {
        let dir = std::env::temp_dir().join(format!("diagram-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("b.shape");
        std::fs::write(&file, "a: A\n").unwrap();
        assert!(versions(&file).is_empty());
        assert!(snapshot(&file, "a: A\n", "Ana Bell"));
        assert!(
            !snapshot(&file, "a: B\n", "Ana Bell"),
            "not again within minutes"
        );
        let v = versions(&file);
        assert_eq!((v.len(), v[0].author.as_str()), (1, "Ana Bell"));
        assert_eq!(version(&file, &v[0].id).unwrap(), "a: A\n");
        assert!(version(&file, "../b.shape").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
