//! The AI harness: any command that reads a prompt on stdin and writes the
//! answer to stdout. Usually the CLI of an AI subscription the person
//! already has, signed in: `--ai claude` (Claude Code), `codex` (Codex CLI,
//! ChatGPT plan), `gemini` (Gemini CLI); `claude:sonnet` picks a model.
//! Any other text is a command line. Chosen with `--ai` or `SHAPE_AI`;
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
pub fn search_path() -> std::ffi::OsString {
    use std::path::PathBuf;
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let home = crate::home_dir().unwrap_or_default();
    let mut more: Vec<PathBuf> = [
        ".local/bin",
        ".claude/local",
        ".npm-global/bin",
        ".bun/bin",
        ".volta/bin",
    ]
    .iter()
    .map(|d| home.join(d))
    .collect();
    if cfg!(windows) {
        // npm's global commands (claude.cmd, codex.cmd, gemini.cmd)
        more.extend(std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("npm")));
    } else {
        more.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    for d in more {
        if !dirs.contains(&d) && d.is_dir() {
            dirs.push(d);
        }
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

/// `cmd` run by the shell: `sh -c`, and `cmd /C` on Windows (which has no
/// sh; it also finds `claude.cmd` and the like that npm installs).
#[cfg(windows)]
fn sh(cmd: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = crate::command("cmd");
    c.args(["/D", "/S", "/C"])
        .raw_arg(format!("\"{cmd}\""))
        .env("PATH", search_path());
    c
}

#[cfg(not(windows))]
fn sh(cmd: &str) -> Command {
    let mut c = crate::command("sh");
    c.args(["-c", cmd]).env("PATH", search_path());
    c
}

/// The subscription CLIs that are installed.
pub fn installed() -> Vec<&'static str> {
    PRESETS
        .iter()
        .filter(|(name, _)| {
            sh(&if cfg!(windows) {
                format!("where {name}")
            } else {
                format!("command -v {name}")
            })
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
        format!("`{prog}` was not found. Install it, or pass its full path: `shape FILE --ai \"/full/path/to/{prog} …\"` (`command -v {prog}` in a terminal prints it).")
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
    // DIAGRAM_AI: the name before the rename to Shape
    if let Ok(c) = std::env::var("SHAPE_AI").or_else(|_| std::env::var("DIAGRAM_AI")) {
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
    run_env(cmd, prompt, &[])
}

/// An image the person gave with a request: a file name and its bytes.
pub struct Image {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// An image from a `data:image/png;base64,…` URI.
pub fn image_from_data_uri(name: &str, uri: &str) -> Option<Image> {
    let (head, data) = uri.split_once(',')?;
    if !head.starts_with("data:image/") || !head.ends_with(";base64") {
        return None;
    }
    let ext = head["data:image/".len()..head.len() - ";base64".len()]
        .replace("svg+xml", "svg")
        .replace("jpeg", "jpg");
    let stem: String = name
        .chars()
        .take_while(|c| *c != '.')
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(40)
        .collect();
    let stem = if stem.is_empty() {
        "image".to_string()
    } else {
        stem
    };
    Some(Image {
        name: format!("{stem}.{ext}"),
        bytes: unbase64(data)?,
    })
}

fn unbase64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

fn quote(p: &std::path::Path) -> String {
    if cfg!(windows) {
        format!("\"{}\"", p.display())
    } else {
        format!("'{}'", p.display().to_string().replace('\'', "'\\''"))
    }
}

/// `cmd` and `prompt` with images handed over the way each CLI takes them:
/// Claude Code reads the files (`--add-dir`, the Read tool), Codex attaches
/// them (`-i FILE`), Gemini includes them (`@FILE` in the prompt); any other
/// command gets their paths in `SHAPE_AI_IMAGES` (one per line). The prompt
/// always lists them.
fn with_images(
    cmd: &str,
    prompt: &str,
    dir: &std::path::Path,
    files: &[std::path::PathBuf],
) -> (String, String, Vec<(String, String)>) {
    let prog = cmd
        .split_whitespace()
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    let gemini = prog == "gemini";
    let lines: Vec<String> = files
        .iter()
        .map(|f| {
            if gemini {
                format!("- @{}", f.display())
            } else {
                format!("- {}", f.display())
            }
        })
        .collect();
    let prompt = format!(
        "{prompt}\n\n## Images\n\nThe person also gave you {} with this; look at {} before you answer, {} part of what they tell you or ask:\n{}\n",
        if files.len() == 1 { "an image".to_string() } else { format!("{} images", files.len()) },
        if files.len() == 1 { "it" } else { "them" },
        if files.len() == 1 { "it is" } else { "they are" },
        lines.join("\n")
    );
    let cmd = match prog {
        "claude" => format!("{cmd} --add-dir {} --allowedTools Read", quote(dir)),
        "codex" => {
            let flags: String = files.iter().map(|f| format!(" -i {}", quote(f))).collect();
            match cmd.strip_suffix(" -") {
                Some(base) => format!("{base}{flags} -"),
                None => format!("{cmd}{flags}"),
            }
        }
        _ => cmd.to_string(),
    };
    let paths = files
        .iter()
        .map(|f| f.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    (cmd, prompt, vec![("SHAPE_AI_IMAGES".into(), paths)])
}

/// [`run`], with images that go along (in a temporary folder, removed after).
pub fn run_with_images(cmd: &str, prompt: &str, images: &[Image]) -> Result<String, String> {
    if images.is_empty() {
        return run(cmd, prompt);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("shape-ai-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("images: {e}"))?;
    let mut files = Vec::new();
    for (i, img) in images.iter().enumerate() {
        let f = dir.join(format!("{}-{}", i + 1, img.name));
        std::fs::write(&f, &img.bytes).map_err(|e| format!("images: {e}"))?;
        files.push(f);
    }
    let (cmd2, prompt2, env) = with_images(cmd, prompt, &dir, &files);
    let out = run_env(&cmd2, &prompt2, &env);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

fn run_env(cmd: &str, prompt: &str, env: &[(String, String)]) -> Result<String, String> {
    let mut c = sh(cmd);
    for (k, v) in env {
        c.env(k, v);
    }
    let mut child = c
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run `{cmd}`: {e}"))?;
    // a command that ends before reading it all (not found, not signed
    // in) closes the pipe: how it ended says more than the broken pipe
    if let Err(e) = child.stdin.take().unwrap().write_all(prompt.as_bytes()) {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            return Err(format!("`{cmd}`: {e}"));
        }
    }
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
        // sh's code for it, and cmd's
        if out.status.code() == Some(127) || (cfg!(windows) && out.status.code() == Some(9009)) {
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
    fn images_go_the_way_each_cli_takes_them() {
        let png =
            image_from_data_uri("Pricing slide.png", "data:image/png;base64,aGVsbG8hPw==").unwrap();
        assert_eq!(
            (png.name.as_str(), png.bytes.as_slice()),
            ("Pricingslide.png", &b"hello!?"[..])
        );
        assert!(image_from_data_uri("x", "data:text/plain;base64,aGk=").is_none());
        let dir = std::path::Path::new("/tmp/shape-ai-1");
        let files = vec![dir.join("1-a.png")];
        let (cmd, prompt, _) = with_images("claude -p", "Hi", dir, &files);
        assert_eq!(
            cmd,
            "claude -p --add-dir '/tmp/shape-ai-1' --allowedTools Read"
        );
        assert!(prompt.ends_with("## Images\n\nThe person also gave you an image with this; look at it before you answer, it is part of what they tell you or ask:\n- /tmp/shape-ai-1/1-a.png\n"));
        let (cmd, _, _) = with_images("codex exec --skip-git-repo-check -", "Hi", dir, &files);
        assert_eq!(
            cmd,
            "codex exec --skip-git-repo-check -i '/tmp/shape-ai-1/1-a.png' -"
        );
        let (cmd, prompt, _) = with_images("gemini", "Hi", dir, &files);
        assert_eq!(cmd, "gemini");
        assert!(prompt.contains("- @/tmp/shape-ai-1/1-a.png"));
        let (_, _, env) = with_images("my-model --fast", "Hi", dir, &files);
        assert_eq!(
            env[0],
            (
                "SHAPE_AI_IMAGES".to_string(),
                "/tmp/shape-ai-1/1-a.png".to_string()
            )
        );
    }

    #[test]
    fn failures_come_with_what_to_try() {
        assert!(hint("claude -p", "`claude -p`: command not found").contains("not found"));
        assert!(hint("claude -p", "Invalid API key · Please run /login").contains("Run `claude`"));
        assert!(hint("codex exec -", "error: not logged in").contains("codex login"));
        assert!(hint("gemini", "429 Too Many Requests").contains("limit"));
        assert!(hint("my-ai", "boom").contains("echo 'Say OK' | my-ai"));
        // ended before reading the prompt: what happened, not a broken pipe
        for _ in 0..20 {
            let e = run("exit 127", &"x".repeat(1 << 20)).unwrap_err();
            assert!(e.contains("command not found"), "{e}");
        }
        assert_eq!(run("cat", "hi").unwrap(), "hi");
    }
}
