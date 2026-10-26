//! Agent skills shipped inside the binary (`shape skills install`).
//!
//! An installed skill ends with a marker: the Shape version that wrote it
//! and a fingerprint of its text. When a newer Shape runs and finds one it
//! wrote that nobody has edited since, it writes the new version over it
//! ([`refresh`]); a skill edited by hand is left as it is.
use std::path::{Path, PathBuf};

macro_rules! skill {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../skills/",
                $name,
                "/SKILL.md"
            )),
        )
    };
}

pub const SKILLS: &[(&str, &str)] = &[
    skill!("shape-draw"),
    skill!("shape-review"),
    skill!("shape-edit"),
    skill!("shape-mindmap"),
    skill!("shape-embed"),
];

pub fn run(args: &[String], global: bool, dir: Option<PathBuf>) -> Result<(), String> {
    match args.first().map(|s| s.as_str()) {
        None | Some("list") => {
            for (name, body) in SKILLS {
                let desc = body
                    .lines()
                    .find_map(|l| l.strip_prefix("description:"))
                    .unwrap_or("")
                    .trim();
                println!("{name:16} {desc}");
            }
            println!("\nInstall with: shape skills install [--global]");
            Ok(())
        }
        Some("show") => {
            let name = args.get(1).ok_or("usage: shape skills show NAME")?;
            let (_, body) = SKILLS
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| format!("unknown skill {name}"))?;
            print!("{body}");
            Ok(())
        }
        Some("install") => {
            let base = match dir {
                Some(d) => d,
                None if global => crate::home_dir()
                    .ok_or("no home folder (HOME or USERPROFILE)")?
                    .join(".claude")
                    .join("skills"),
                None => crate::repo_root()
                    .unwrap_or(std::env::current_dir().map_err(|e| e.to_string())?)
                    .join(".claude/skills"),
            };
            for (name, body) in SKILLS {
                // the same skill from before the rename to Shape (`diagram-draw`…)
                let old = base.join(name.replacen("shape-", "diagram-", 1));
                if old.join("SKILL.md").is_file() {
                    let _ = std::fs::remove_dir_all(&old);
                }
                let dir = base.join(name);
                std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                std::fs::write(dir.join("SKILL.md"), stamped(body)).map_err(|e| e.to_string())?;
                println!("installed {}", dir.join("SKILL.md").display());
            }
            Ok(())
        }
        Some(other) => Err(format!(
            "unknown skills command {other:?} (list | show NAME | install [--global] [--dir PATH])"
        )),
    }
}

const MARK: &str = "<!-- shape-skill ";

/// The fingerprints of every skill text Shape shipped before installed
/// skills had a marker (from the repository's history): a copy without a
/// marker is Shape's, untouched, only when it is one of these.
const SHIPPED: &[&str] = &[
    "0611288cfb124119",
    "09b9741578b88da9",
    "12b5d8ca88083fb0",
    "1372c590af429748",
    "19002c15e06288ef",
    "2f71826953d972ac",
    "3060b21c4503bed7",
    "380d8309455d5187",
    "3eade37c2198abf9",
    "55b4a51ded223f93",
    "6168fa8ea0049349",
    "663ff327b1bbb1f6",
    "69fdbf1c5bdd768f",
    "6c6e6222d026af38",
    "70310af1dabc147e",
    "7186031219a45d2c",
    "77533ef97a7dd012",
    "78ef9943f2389e5f",
    "9c4cf98cb7d331bf",
    "b7bfc37496350a6f",
    "c4342ad9f13210fb",
    "c5f850e8a86da6b1",
    "c8b3103f8bc23f88",
    "ca841dfa7b11e9e6",
    "ce042013635f2e80",
    "d298365632524fc1",
    "d33f1a252604490e",
    "d910fb658e3b9792",
    "ddd3ce2c476e9d1f",
    "f1da6c35987b602e",
];

/// A short fingerprint of a text (FNV-1a), to tell whether it was edited.
fn fingerprint(text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// A skill as installed: its text, then the marker.
fn stamped(body: &str) -> String {
    format!(
        "{body}\n{MARK}{} {} -->\n<!-- Installed by `shape skills install`. A newer Shape updates this file, unless you edit it. -->\n",
        env!("CARGO_PKG_VERSION"),
        fingerprint(body)
    )
}

/// An installed skill's text and the fingerprint its marker has (None for
/// one installed before markers).
fn read_back(text: &str) -> (String, Option<String>) {
    match text.rfind(MARK) {
        Some(i) => {
            let fp = text[i + MARK.len()..]
                .split_whitespace()
                .nth(1)
                .map(String::from);
            (
                text[..i]
                    .strip_suffix('\n')
                    .unwrap_or(&text[..i])
                    .to_string(),
                fp,
            )
        }
        None => (text.to_string(), None),
    }
}

/// Where skills are installed: the global folder, and the nearest
/// `.claude/skills` above the working directory.
fn places() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = crate::home_dir() {
        out.push(home.join(".claude").join("skills"));
    }
    if let Ok(mut dir) = std::env::current_dir() {
        loop {
            let s = dir.join(".claude/skills");
            if s.is_dir() {
                if !out.contains(&s) {
                    out.push(s);
                }
                break;
            }
            if !dir.pop() {
                break;
            }
        }
    }
    out
}

/// Bring the Shape skills installed in `base` up to date: those this or an
/// older Shape wrote, and nobody edited. Returns the ones it rewrote.
fn refresh_in(base: &Path) -> Vec<String> {
    let mut done = Vec::new();
    for (name, body) in SKILLS {
        let file = base.join(name).join("SKILL.md");
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let (was, fp) = read_back(&text);
        let ours = match &fp {
            Some(fp) => *fp == fingerprint(&was),
            // from before the markers: as some Shape wrote it, untouched
            None => {
                SHIPPED.contains(&fingerprint(&text).as_str())
                    || SHIPPED.contains(&fingerprint(text.trim_end()).as_str())
            }
        };
        if !ours {
            continue;
        }
        if text == stamped(body) {
            continue;
        }
        if std::fs::write(&file, stamped(body)).is_ok() && was.trim() != body.trim() {
            done.push(name.to_string());
        }
    }
    done
}

/// After an upgrade: the installed Shape skills, up to date (see the module
/// docs); says which it updated. `SHAPE_NO_SKILL_REFRESH=1` (or `CI`) turns it off.
pub fn refresh() {
    let set = |k: &str| std::env::var(k).is_ok_and(|v| !v.is_empty() && v != "0");
    if set("SHAPE_NO_SKILL_REFRESH") || set("CI") {
        return;
    }
    for base in places() {
        let done = refresh_in(&base);
        if !done.is_empty() {
            eprintln!(
                "shape: updated the Claude skills {} in {} to Shape {}",
                done.join(", "),
                base.display(),
                env!("CARGO_PKG_VERSION")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_skills_follow_upgrades_unless_edited() {
        let dir = std::env::temp_dir().join(format!("shape-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (name, body) = SKILLS[0];
        let file = dir.join(name).join("SKILL.md");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        // an older Shape's copy, unedited: brought up to date
        let old = body.replace("Steps", "Old steps");
        std::fs::write(&file, stamped(&old)).unwrap();
        assert_eq!(refresh_in(&dir), vec![name.to_string()]);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), stamped(body));
        // up to date: nothing to do
        assert!(refresh_in(&dir).is_empty());
        // edited by hand: left alone
        let edited = stamped(&old).replacen("# ", "# My ", 1);
        std::fs::write(&file, &edited).unwrap();
        assert!(refresh_in(&dir).is_empty());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), edited);
        // from before the markers, edited: left alone
        std::fs::write(&file, &old).unwrap();
        assert!(refresh_in(&dir).is_empty());
        // from before the markers, as an earlier Shape shipped it: updated
        let shipped = crate::command("git")
            .args(["show", &format!("v0.60.0:skills/{name}/SKILL.md")])
            .output()
            .unwrap()
            .stdout;
        if !shipped.is_empty() {
            std::fs::write(&file, &shipped).unwrap();
            assert_eq!(refresh_in(&dir), vec![name.to_string()]);
            assert_eq!(std::fs::read_to_string(&file).unwrap(), stamped(body));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
