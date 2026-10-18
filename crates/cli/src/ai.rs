//! The AI harness: any command that reads a prompt on stdin and writes the
//! answer to stdout. Usually the CLI of an AI subscription the person
//! already has, signed in: `--ai claude` (Claude Code), `codex` (Codex CLI,
//! ChatGPT plan), `gemini` (Gemini CLI); `claude:sonnet` picks a model.
//! Any other text is a command line. Chosen with `--ai` or `DIAGRAM_AI`;
//! when neither is set, the first of those CLIs that is installed.
use std::io::Write;
use std::process::{Command, Stdio};

/// The subscription CLIs, in the order they are looked for: (name, the
/// command that reads a prompt on stdin and prints the answer).
pub const PRESETS: &[(&str, &str)] = &[
    ("claude", "claude -p"),
    ("codex", "codex exec --skip-git-repo-check -"),
    ("gemini", "gemini"),
];

/// A preset's command (`claude`, `claude:sonnet`), or the text as a command.
pub fn resolve(spec: &str) -> String {
    let spec = spec.trim();
    let (name, model) = match spec.split_once(':') {
        Some((n, m)) if !n.contains(' ') && !m.contains(' ') => (n, Some(m)),
        _ => (spec, None),
    };
    match PRESETS.iter().find(|(n, _)| *n == name) {
        Some((_, cmd)) => match (name, model) {
            ("claude", Some(m)) => format!("claude -p --model {m}"),
            ("codex", Some(m)) => format!("codex exec --skip-git-repo-check -m {m} -"),
            ("gemini", Some(m)) => format!("gemini -m {m}"),
            _ => cmd.to_string(),
        },
        None => spec.to_string(),
    }
}

/// Where the subscription CLIs are looked for: the PATH, and the places
/// their installers put them (an editor opened from Finder or a desktop
/// launcher does not get the shell's PATH).
pub fn search_path() -> String {
    let mut dirs: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .filter(|d| !d.is_empty())
        .map(String::from)
        .collect();
    let home = std::env::var("HOME").unwrap_or_default();
    let more = [
        format!("{home}/.local/bin"),
        format!("{home}/.claude/local"),
        format!("{home}/.npm-global/bin"),
        format!("{home}/.bun/bin"),
        format!("{home}/.volta/bin"),
        "/opt/homebrew/bin".into(),
        "/usr/local/bin".into(),
    ];
    for d in more {
        if !dirs.contains(&d) && std::path::Path::new(&d).is_dir() {
            dirs.push(d);
        }
    }
    dirs.join(":")
}

fn sh(cmd: &str) -> Command {
    let mut c = Command::new("sh");
    c.args(["-c", cmd]).env("PATH", search_path());
    c
}

/// The subscription CLIs that are installed.
pub fn installed() -> Vec<&'static str> {
    PRESETS
        .iter()
        .filter(|(name, _)| {
            sh(&format!("command -v {name}"))
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .map(|(name, _)| *name)
        .collect()
}

/// What to try when the harness failed with `error`.
pub fn hint(cmd: &str, error: &str) -> String {
    let e = error.to_lowercase();
    let prog = cmd.split_whitespace().next().unwrap_or(cmd);
    if e.contains("not found") || e.contains("no such file") || e.contains("cannot run") {
        format!("`{prog}` was not found. Install it, or pass its full path: `diagram FILE --ai \"/full/path/to/{prog} …\"` (`command -v {prog}` in a terminal prints it).")
    } else if [
        "login",
        "log in",
        "logged in",
        "sign in",
        "auth",
        "401",
        "403",
        "api key",
        "credential",
        "token",
    ]
    .iter()
    .any(|w| e.contains(w))
    {
        format!(
            "`{prog}` is not signed in. Run `{}` in a terminal and sign in, then try again.",
            match prog {
                "claude" => "claude",
                "codex" => "codex login",
                "gemini" => "gemini",
                other => other,
            }
        )
    } else if [
        "rate limit",
        "429",
        "usage limit",
        "quota",
        "limit reached",
        "overloaded",
        "529",
    ]
    .iter()
    .any(|w| e.contains(w))
    {
        "Your subscription's limit is reached or the service is busy. Wait a little, or use another one (`--ai codex`, `--ai gemini`).".into()
    } else if e.contains("timed out") || e.contains("network") || e.contains("connect") {
        "The model could not be reached. Check your connection (and a VPN or proxy), then try again.".into()
    } else {
        format!("Try it in a terminal to see what it says: `echo 'Say OK' | {cmd}`.")
    }
}

pub fn command(explicit: Option<&str>) -> Option<String> {
    if let Some(c) = explicit.filter(|c| !c.trim().is_empty()) {
        return Some(resolve(c));
    }
    if let Ok(c) = std::env::var("DIAGRAM_AI") {
        return Some(resolve(&c)).filter(|c| !c.trim().is_empty());
    }
    // the first subscription CLI that is installed
    let found = installed();
    PRESETS
        .iter()
        .find(|(name, _)| found.contains(name))
        .map(|(_, cmd)| cmd.to_string())
}

pub fn run(cmd: &str, prompt: &str) -> Result<String, String> {
    let mut child = sh(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run `{cmd}`: {e}"))?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(prompt.as_bytes())
        .map_err(|e| format!("`{cmd}`: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("`{cmd}`: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        // some CLIs say what went wrong on stdout
        let err = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).into_owned()
        } else {
            err.into_owned()
        };
        if out.status.code() == Some(127) {
            return Err(format!("`{cmd}`: command not found"));
        }
        return Err(format!("`{cmd}` failed: {}", err.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_come_with_what_to_try() {
        assert!(hint("claude -p", "`claude -p`: command not found").contains("not found"));
        assert!(hint("claude -p", "Invalid API key · Please run /login").contains("Run `claude`"));
        assert!(hint("codex exec -", "error: not logged in").contains("codex login"));
        assert!(hint("gemini", "429 Too Many Requests").contains("limit"));
        assert!(hint("my-ai", "boom").contains("echo 'Say OK' | my-ai"));
        let e = run("exit 127", "x").unwrap_err();
        assert!(e.contains("command not found"), "{e}");
        assert_eq!(run("cat", "hi").unwrap(), "hi");
    }
}
