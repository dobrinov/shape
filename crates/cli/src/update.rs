//! Is there a newer Shape? The editor asks GitHub for the latest release in
//! the background (through `curl`) when it starts and every hour while it
//! runs; the answer is cached for an hour (one older than this copy is not
//! believed), and a failure (offline, rate limit) is quiet.
//! `SHAPE_NO_UPDATE_CHECK=1` turns it off; so does `CI`.
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const REPO: &str = "dobrinov/shape";
/// How long an answer is kept, and how often a running editor asks again.
const HOUR: u64 = 60 * 60;

/// A release newer than this binary.
#[derive(Debug, Clone)]
pub struct Update {
    pub latest: String,
    /// How to get it: `brew upgrade shape`, or where to download it.
    pub how: String,
    /// The release, with its notes.
    pub url: String,
}

/// The answer of the checks [`start`] runs: `None` until the first is done,
/// then `Some(None)` when this is the latest (or the check is off or failed).
static FOUND: Mutex<Option<Option<Update>>> = Mutex::new(None);

/// Look for a newer release in the background, now and every hour; say so
/// on stderr when one turns up.
pub fn start() {
    std::thread::spawn(|| {
        let mut told = String::new();
        loop {
            let found = check();
            if let Some(u) = found.as_ref().filter(|u| u.latest != told) {
                eprintln!(
                    "shape: {} is out (you have {}): {}",
                    u.latest,
                    env!("CARGO_PKG_VERSION"),
                    u.how
                );
                told = u.latest.clone();
            }
            if let Ok(mut f) = FOUND.lock() {
                *f = Some(found);
            }
            if off() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(HOUR));
        }
    });
}

/// Check now, whatever the cache says (Shape → Check for Updates…): the
/// newer release, `None` when this is the latest, or why it could not tell.
pub fn check_now() -> Result<Option<Update>, String> {
    let latest = fetch().ok_or("Could not reach GitHub to check (offline, or its limit on checks for now). Try again in a while.")?;
    if let Some(f) = cache_file() {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = std::fs::write(f, format!("{now} {latest}\n"));
    }
    let found = newer(&latest, env!("CARGO_PKG_VERSION")).then(|| Update {
        url: format!("https://github.com/{REPO}/releases/tag/v{latest}"),
        how: how(),
        latest,
    });
    // the editor's badge follows
    if let Ok(mut f) = FOUND.lock() {
        *f = Some(found.clone());
    }
    Ok(found)
}

/// What the checks found, once the first is done.
pub fn found() -> Option<Option<Update>> {
    FOUND.lock().ok().and_then(|f| f.clone())
}

fn check() -> Option<Update> {
    if off() {
        return None;
    }
    let latest = latest()?;
    newer(&latest, env!("CARGO_PKG_VERSION")).then(|| Update {
        url: format!("https://github.com/{REPO}/releases/tag/v{latest}"),
        how: how(),
        latest,
    })
}

fn off() -> bool {
    let set = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty() && v != "0");
    set("SHAPE_NO_UPDATE_CHECK") || set("DIAGRAM_NO_UPDATE_CHECK") || set("CI")
}

/// The latest release's version: from the cache when it is less than an hour old.
fn latest() -> Option<String> {
    let file = cache_file();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Some(cached) = file
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| fresh(&s, now, env!("CARGO_PKG_VERSION")))
    {
        return Some(cached);
    }
    let v = fetch()?;
    if let Some(f) = &file {
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(f, format!("{now} {v}\n"));
    }
    Some(v)
}

/// The version in a cache file (`SECONDS VERSION`) written less than an hour
/// before `now`, unless it is older than `current` (this copy is newer than
/// what was cached: the cache is out of date).
fn fresh(cache: &str, now: u64, current: &str) -> Option<String> {
    let (at, v) = cache.trim().split_once(' ')?;
    let at: u64 = at.parse().ok()?;
    (now.saturating_sub(at) < HOUR && !v.is_empty() && !newer(current, v)).then(|| v.to_string())
}

fn fetch() -> Option<String> {
    let out = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            "5",
            "-H",
            "Accept: application/vnd.github+json",
        ])
        .arg(format!(
            "https://api.github.com/repos/{REPO}/releases?per_page=30"
        ))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    highest(&serde_json::from_slice(&out.stdout).ok()?)
}

/// The highest released version. Not GitHub's "latest", which is the release
/// created last: not the newest when two tags are built at once.
fn highest(releases: &serde_json::Value) -> Option<String> {
    releases
        .as_array()?
        .iter()
        .filter(|r| {
            !r["draft"].as_bool().unwrap_or(false) && !r["prerelease"].as_bool().unwrap_or(false)
        })
        .filter_map(|r| r["tag_name"].as_str())
        .map(|t| t.trim_start_matches('v').to_string())
        .reduce(|a, b| if newer(&b, &a) { b } else { a })
}

fn cache_file() -> Option<PathBuf> {
    let env = |k: &str| {
        std::env::var_os(k)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    let dir = env("XDG_CACHE_HOME").or_else(|| {
        if cfg!(windows) {
            env("LOCALAPPDATA")
        } else if cfg!(target_os = "macos") {
            env("HOME").map(|h| h.join("Library/Caches"))
        } else {
            env("HOME").map(|h| h.join(".cache"))
        }
    })?;
    Some(dir.join("shape").join("latest-version"))
}

/// Whether version `a` comes after `b` (`x.y.z`; a missing part is 0).
fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.split(['.', '-'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parts(a) > parts(b)
}

/// How this copy was installed decides how to upgrade it.
fn how() -> String {
    let brew = std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok())
        .is_some_and(|p| p.to_string_lossy().contains("/Cellar/"));
    // where the site's installers put it: running one again upgrades
    let exe = std::env::current_exe().unwrap_or_default();
    let installed = |tail: &[&str]| {
        let parts: Vec<_> = exe.components().rev().skip(1).take(tail.len()).collect();
        parts
            .iter()
            .rev()
            .map(|c| c.as_os_str())
            .eq(tail.iter().map(std::ffi::OsStr::new))
    };
    if brew {
        "brew upgrade shape".into()
    } else if cfg!(windows) && installed(&["Programs", "Shape"]) {
        "irm https://dobrinov.github.io/shape/install.ps1 | iex".into()
    } else if !cfg!(windows) && installed(&[".local", "bin"]) {
        "curl -fsSL https://dobrinov.github.io/shape/install.sh | sh".into()
    } else {
        format!("https://github.com/{REPO}/releases/latest")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.34.0", "0.33.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.0"));
        assert!(!newer("0.33.0", "0.33.0"));
        assert!(!newer("0.32.1", "0.33.0"));
    }

    #[test]
    fn the_highest_release_wins() {
        let r = serde_json::json!([
            {"tag_name": "v0.34.1", "draft": false, "prerelease": false},
            {"tag_name": "v0.35.0", "draft": false, "prerelease": false},
            {"tag_name": "v0.36.0", "draft": true, "prerelease": false},
            {"tag_name": "v0.9.0", "draft": false, "prerelease": false}
        ]);
        assert_eq!(highest(&r).as_deref(), Some("0.35.0"));
        assert_eq!(
            highest(&serde_json::json!({"message": "rate limited"})),
            None
        );
    }

    #[test]
    fn the_cache_lasts_an_hour_and_never_trails_this_copy() {
        assert_eq!(
            fresh("1000 0.34.0\n", 1000 + HOUR - 1, "0.34.0").as_deref(),
            Some("0.34.0")
        );
        assert_eq!(fresh("1000 0.34.0\n", 1000 + HOUR, "0.34.0"), None);
        // cached before this copy came out: ask again
        assert_eq!(fresh("1000 0.53.1\n", 1001, "0.61.0"), None);
        assert_eq!(fresh("garbage", 1000, "0.1.0"), None);
        assert_eq!(fresh("", 1000, "0.1.0"), None);
    }
}
