//! Local HTTP server for the editor: serves the web app, reads and writes the
//! board file, and reports external changes (the editor polls for them).
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::assets;

pub const FIRST_PORT: u16 = 5591;

fn bind(port: Option<u16>) -> Result<(Server, u16), String> {
    if let Some(p) = port {
        return Server::http(("127.0.0.1", p))
            .map(|s| (s, p))
            .map_err(|e| format!("cannot listen on port {p}: {e}"));
    }
    for p in FIRST_PORT..FIRST_PORT + 100 {
        if let Ok(s) = Server::http(("127.0.0.1", p)) {
            return Ok((s, p));
        }
    }
    Err("no free port found".into())
}

fn mtime(p: &Path) -> u128 {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn query_param(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        (k == key).then(|| url_decode(v))
    })
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
                {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Where links between boards may lead: the git repository the board is in,
/// or its folder.
fn board_root(file: &Path) -> PathBuf {
    let abs = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let dir = abs.parent().unwrap_or(Path::new(".")).to_path_buf();
    std::process::Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .and_then(|p| std::fs::canonicalize(p).ok())
        .unwrap_or(dir)
}

/// A path relative to the root, with `/` separators.
fn rel_to(root: &Path, p: &Path) -> String {
    let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    abs.strip_prefix(root)
        .map(|r| r.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| p.display().to_string())
}

/// The board a request is about: `?file=REL` (another board under the root,
/// reached by a link) or the one the server was started with.
fn target(url: &str, root: &Path) -> Option<Result<(PathBuf, String), String>> {
    let rel = query_param(url, "file").filter(|r| !r.is_empty())?;
    if !shape_core::model::is_board_file(&rel) {
        return Some(Err(format!("{rel}: only .shape boards open here")));
    }
    Some(match std::fs::canonicalize(root.join(&rel)) {
        Ok(p) if p.starts_with(root) => Ok((p, rel)),
        Ok(_) => Err(format!("{rel} is outside {}", root.display())),
        Err(_) => Err(format!("{rel} does not exist")),
    })
}

/// Database schema files under the root (`structure.sql`, `schema.rb`,
/// other `.sql` files), as paths from `dir`.
fn schema_files(root: &Path, dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if name.ends_with(".sql") || name == "schema.rb" {
                out.push(crate::relative_to(dir, &p));
            }
        }
        if out.len() > 200 {
            break;
        }
    }
    // the usual places first
    out.sort_by_key(|f| (!(f.ends_with("structure.sql") || f.ends_with("schema.rb")), f.clone()));
    out
}

/// The boards under the root, for linking to one (hidden folders, `target`
/// and `node_modules` left out).
fn boards(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if shape_core::model::is_board_file(&name) && out.len() < 500 {
                out.push(rel_to(root, &p));
            }
        }
    }
    out.sort();
    out
}

/// Git's three versions of a file it could not merge (base, yours, theirs:
/// index stages 1–3), when a merge left it unmerged.
fn unmerged(file: &Path) -> Option<(String, String, String)> {
    let dir = file
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = format!("./{}", file.file_name()?.to_string_lossy());
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    let listed = git(&["ls-files", "-u", "--", &name])?;
    if listed.trim().is_empty() {
        return None;
    }
    let stage = |n: u8| git(&["show", &format!(":{n}:{name}")]).unwrap_or_default();
    Some((stage(1), stage(2), stage(3)))
}

fn respond_json(req: Request, status: u16, v: Value) {
    let h = Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap();
    let _ = req.respond(
        Response::from_string(v.to_string())
            .with_status_code(status)
            .with_header(h),
    );
}

pub fn open_browser(url: &str) {
    let cmd = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(target_os = "windows") {
        ("cmd", vec!["/C", "start", "", url])
    } else {
        ("xdg-open", vec![url])
    };
    let _ = std::process::Command::new(cmd.0)
        .args(cmd.1)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Where the editor shows up once the server listens.
pub enum Show {
    /// nowhere: the server only (`--no-open`)
    None,
    /// a browser tab (`--browser`, or when there is no window to be had)
    Browser,
    /// the native window; the server runs beside it and stops when it closes
    Window(crate::window::Native),
}

pub fn serve(
    file: PathBuf,
    port: Option<u16>,
    show: Show,
    ai: Option<String>,
    idle_exit: bool,
) -> Result<(), String> {
    let (server, port) = bind(port)?;
    let url = format!("http://127.0.0.1:{port}/");
    let display = file.display().to_string();
    let stop = if matches!(show, Show::Window(_)) {
        "close the window to stop"
    } else {
        "Ctrl-C to stop"
    };
    eprintln!("shape: editing {display} at {url}  ({stop})");
    match &ai {
        Some(c) => eprintln!("shape: ✦ AI via `{c}`"),
        None => {
            eprintln!("shape: no AI harness (use --ai CMD); ✦ actions will show the prompt to copy")
        }
    }
    crate::update::start();
    match show {
        Show::None => listen(server, file, display, ai, idle_exit),
        Show::Browser => {
            open_browser(&url);
            listen(server, file, display, ai, idle_exit)
        }
        Show::Window(window) => {
            let name = crate::window::board_name(&file);
            // the window's own lifetime ends the process: no idle timer
            let listener = std::thread::spawn(move || {
                if let Err(e) = listen(server, file, display, ai, false) {
                    eprintln!("shape: {e}");
                    std::process::exit(1);
                }
            });
            // returns only when no window could be made: a tab, then
            if let Err(e) = window.run(&url, &name) {
                eprintln!("shape: no window ({e}); opening the browser");
                open_browser(&url);
            }
            listener
                .join()
                .map_err(|_| "the server stopped".to_string())
        }
    }
}

/// Answer the editor's requests until the process ends.
fn listen(
    server: Server,
    file: PathBuf,
    display: String,
    ai: Option<String>,
    idle_exit: bool,
) -> Result<(), String> {
    // started from a double-click (`shape install-app`): no terminal to stop
    // it, so it stops once the editor tabs are gone (they check in every 30s)
    let heard = std::sync::Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    if idle_exit {
        let heard = heard.clone();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(15));
            if heard
                .lock()
                .map(|t| t.elapsed().as_secs() > 180)
                .unwrap_or(false)
            {
                eprintln!("shape: no editor open for 3 minutes; stopping");
                std::process::exit(0);
            }
        });
    }
    let author = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        // git's name only: without one, the editor asks who you are
        .filter(|s| !s.is_empty());
    let root = board_root(&file);
    let start = (file.clone(), display.clone());
    for mut req in server.incoming_requests() {
        if let Ok(mut t) = heard.lock() {
            *t = std::time::Instant::now();
        }
        let url = req.url().to_string();
        let path = url.split('?').next().unwrap_or("/").to_string();
        // `?file=` picks another board (links between boards); default: the one we started with
        let (file, display) = match target(&url, &root) {
            None => start.clone(),
            Some(Ok(t)) => t,
            Some(Err(e)) => {
                respond_json(req, 404, json!({ "ok": false, "error": e }));
                continue;
            }
        };
        match (req.method().clone(), path.as_str()) {
            // a merge that clashed: the three versions, for the editor to settle
            (Method::Get, "/api/conflict") => {
                let v = match unmerged(&file) {
                    Some((base, ours, theirs)) => {
                        json!({ "conflict": true, "git": true, "base": base, "ours": ours, "theirs": theirs })
                    }
                    None => {
                        let text = std::fs::read_to_string(&file).unwrap_or_default();
                        match shape_core::merge::split_markers(&text) {
                            Some((base, ours, theirs)) => {
                                json!({ "conflict": true, "git": false, "base": base.unwrap_or_default(), "ours": ours, "theirs": theirs })
                            }
                            None => json!({ "conflict": false }),
                        }
                    }
                };
                respond_json(req, 200, v)
            }
            // the settled board: written, and marked resolved in git
            (Method::Post, "/api/resolve") => {
                let mut body = String::new();
                if req.as_reader().read_to_string(&mut body).is_err() {
                    respond_json(
                        req,
                        400,
                        json!({ "ok": false, "error": "body is not UTF-8" }),
                    );
                    continue;
                }
                let was = unmerged(&file).is_some();
                if let Err(e) = std::fs::write(&file, &body) {
                    respond_json(req, 500, json!({ "ok": false, "error": e.to_string() }));
                    continue;
                }
                let staged = was
                    && std::process::Command::new("git")
                        .arg("-C")
                        .arg(
                            file.parent()
                                .filter(|d| !d.as_os_str().is_empty())
                                .unwrap_or(Path::new(".")),
                        )
                        .args(["add", "--"])
                        .arg(file.file_name().unwrap_or_default())
                        .status()
                        .is_ok_and(|s| s.success());
                respond_json(
                    req,
                    200,
                    json!({ "ok": true, "staged": staged, "mtime": mtime(&file).to_string() }),
                )
            }
            // a file the board reads (a schema block's), relative to the board
            (Method::Get, "/api/source") => {
                let rel = query_param(&url, "path").unwrap_or_default();
                let dir = file.parent().unwrap_or(&root).to_path_buf();
                match std::fs::canonicalize(dir.join(&rel)) {
                    Ok(p) if p.starts_with(&root) && p.is_file() => match std::fs::read_to_string(&p) {
                        Ok(source) => respond_json(req, 200, json!({ "ok": true, "path": rel, "source": source })),
                        Err(e) => respond_json(req, 500, json!({ "ok": false, "error": format!("{rel}: {e}") })),
                    },
                    Ok(_) => respond_json(req, 403, json!({ "ok": false, "error": format!("{rel} is outside {}", root.display()) })),
                    Err(_) => respond_json(req, 404, json!({ "ok": false, "error": format!("{rel} does not exist") })),
                }
            }
            // database schema files in the project, relative to the board
            (Method::Get, "/api/schemas") => {
                let dir = file.parent().unwrap_or(&root).to_path_buf();
                respond_json(req, 200, json!({ "files": schema_files(&root, &dir) }))
            }
            (Method::Get, "/api/boards") => respond_json(
                req,
                200,
                json!({ "boards": boards(&root), "current": rel_to(&root, &file) }),
            ),
            (Method::Get, "/api/file") => {
                let source = std::fs::read_to_string(&file).unwrap_or_default();
                let name = file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                // a theme file the board names comes along
                let (doc, _) = shape_core::dsl::parse_lenient(&source);
                let themes: serde_json::Map<String, serde_json::Value> =
                    crate::theme_file(&doc, &file.to_string_lossy())
                        .ok()
                        .flatten()
                        .map(|(n, src)| (n, serde_json::Value::String(src)))
                        .into_iter()
                        .collect();
                respond_json(
                    req,
                    200,
                    json!({ "path": display, "rel": rel_to(&root, &file), "name": name, "source": source, "themes": themes, "mtime": mtime(&file).to_string(), "author": author, "format": crate::format_for(&display), "version": env!("CARGO_PKG_VERSION") }),
                );
            }
            (Method::Get, "/api/ai") => respond_json(
                req,
                200,
                json!({ "available": ai.is_some(), "command": ai, "installed": crate::ai::installed(), "presets": crate::ai::PRESETS.iter().map(|p| p.0).collect::<Vec<_>>() }),
            ),
            // a tiny prompt, to see whether the harness works
            (Method::Post, "/api/ai/check") => {
                let Some(cmd) = ai.clone() else {
                    respond_json(
                        req,
                        200,
                        json!({ "ok": false, "error": "no AI is connected" }),
                    );
                    continue;
                };
                std::thread::spawn(move || {
                    match crate::ai::run(&cmd, "Reply with the single word OK.") {
                        Ok(text) => {
                            respond_json(req, 200, json!({ "ok": true, "text": text.trim() }))
                        }
                        Err(e) => respond_json(
                            req,
                            200,
                            json!({ "ok": false, "hint": crate::ai::hint(&cmd, &e), "error": e }),
                        ),
                    }
                });
            }
            (Method::Post, "/api/ai") => {
                let mut prompt = String::new();
                let _ = req.as_reader().read_to_string(&mut prompt);
                let Some(cmd) = ai.clone() else {
                    respond_json(
                        req,
                        200,
                        json!({ "ok": false, "error": "no AI harness: start with --ai CMD or set SHAPE_AI" }),
                    );
                    continue;
                };
                // models take seconds; keep serving saves and polls meanwhile
                std::thread::spawn(move || match crate::ai::run(&cmd, &prompt) {
                    Ok(text) => respond_json(req, 200, json!({ "ok": true, "text": text })),
                    Err(e) => respond_json(
                        req,
                        200,
                        json!({ "ok": false, "hint": crate::ai::hint(&cmd, &e), "error": e }),
                    ),
                });
            }
            (Method::Get, "/api/version") => {
                respond_json(req, 200, json!({ "mtime": mtime(&file).to_string() }))
            }
            (Method::Put, "/api/file") => {
                let mut body = String::new();
                if req.as_reader().read_to_string(&mut body).is_err() {
                    respond_json(
                        req,
                        400,
                        json!({ "ok": false, "error": "body is not UTF-8" }),
                    );
                    continue;
                }
                // DSL files keep the text as typed; JSON / Mermaid files are converted
                let text = match crate::format_for(&display) {
                    "dsl" if shape_core::detect(&body) == shape_core::Format::Dsl => Ok(body),
                    fmt => shape_core::parse(&body)
                        .map_err(|e| {
                            e.iter()
                                .map(|d| d.to_string())
                                .collect::<Vec<_>>()
                                .join("; ")
                        })
                        .and_then(|d| shape_core::export(&d, fmt)),
                };
                let who = req
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("X-Diagram-Author"))
                    .map(|h| url_decode(h.value.as_str()))
                    .filter(|w| !w.trim().is_empty())
                    .or_else(|| author.clone())
                    .unwrap_or_default();
                let written = text.and_then(|t| {
                    std::fs::write(&file, &t).map_err(|e| e.to_string())?;
                    // a board git does not track keeps snapshots instead
                    crate::history::snapshot(&file, &t, &who);
                    Ok(())
                });
                match written {
                    Ok(()) => respond_json(
                        req,
                        200,
                        json!({ "ok": true, "mtime": mtime(&file).to_string() }),
                    ),
                    Err(e) => respond_json(req, 500, json!({ "ok": false, "error": e })),
                }
            }
            (Method::Get, "/api/git") => {
                let reference = query_param(&url, "ref").unwrap_or_else(|| "HEAD".into());
                match crate::git_show_pub(&reference, &display) {
                    Ok(source) => respond_json(
                        req,
                        200,
                        json!({ "ok": true, "source": source, "ref": reference }),
                    ),
                    Err(e) => respond_json(req, 200, json!({ "ok": false, "error": e })),
                }
            }
            // a newer release, once the check is done (see `update`)
            (Method::Get, "/api/update") => {
                let current = env!("CARGO_PKG_VERSION");
                let body = match crate::update::found() {
                    None => json!({ "checked": false, "current": current }),
                    Some(None) => json!({ "checked": true, "current": current }),
                    Some(Some(u)) => {
                        json!({ "checked": true, "current": current, "latest": u.latest, "how": u.how, "url": u.url })
                    }
                };
                respond_json(req, 200, body)
            }
            // the board's history: commits, or snapshots beside a board git does not track
            (Method::Get, "/api/history") => {
                let list: Vec<Value> = crate::history::versions(&file)
                    .iter()
                    .map(|v| v.to_json())
                    .collect();
                respond_json(
                    req,
                    200,
                    json!({ "ok": true, "git": crate::history::in_git(&file), "versions": list }),
                )
            }
            (Method::Get, "/api/history/version") => {
                let id = query_param(&url, "id").unwrap_or_default();
                match crate::history::version(&file, &id) {
                    Ok(source) => respond_json(req, 200, json!({ "ok": true, "source": source })),
                    Err(e) => respond_json(req, 200, json!({ "ok": false, "error": e })),
                }
            }
            // a team's recipes: `recipes/*.shape` up to the repository's root
            (Method::Get, "/api/recipes") => {
                let files: serde_json::Map<String, Value> = crate::recipe_files(&display)
                    .into_iter()
                    .map(|(id, text)| (id, Value::String(text)))
                    .collect();
                respond_json(req, 200, json!({ "ok": true, "recipes": files }))
            }
            // what a rethink takes as the new information (see `git_since`)
            (Method::Get, "/api/since") => {
                let since = crate::git_since_on(&display);
                respond_json(
                    req,
                    200,
                    json!({ "ok": true, "source": since.as_ref().map(|s| &s.0), "label": since.as_ref().map(|s| &s.1), "branch": since.as_ref().map(|s| &s.2), "main": since.as_ref().map(|s| &s.3) }),
                )
            }
            (Method::Get, p) => match assets::get(p) {
                Some((bytes, ct)) => {
                    let h = Header::from_bytes("Content-Type", ct).unwrap();
                    let nc = Header::from_bytes("Cache-Control", "no-cache").unwrap();
                    let _ = req.respond(
                        Response::from_data(bytes.into_owned())
                            .with_header(h)
                            .with_header(nc),
                    );
                }
                None => {
                    let _ = req.respond(Response::from_string("not found").with_status_code(404));
                }
            },
            _ => {
                let _ =
                    req.respond(Response::from_string("method not allowed").with_status_code(405));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn other_boards_stay_under_the_root() {
        let dir = std::env::temp_dir().join(format!("shape-srv-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.shape"), "a\n").unwrap();
        std::fs::write(dir.join("sub/b.diagram"), "b\n").unwrap();
        let root = std::fs::canonicalize(&dir).unwrap();
        let t = |q: &str| super::target(&format!("/api/file?file={q}"), &root);
        assert_eq!(t("sub/b.diagram").unwrap().unwrap().1, "sub/b.diagram");
        assert!(t("sub/../a.shape").unwrap().is_ok());
        assert!(t("../x.shape").unwrap().is_err());
        assert!(t("sub/missing.shape").unwrap().is_err());
        assert!(t("a.json").unwrap().is_err());
        assert!(super::target("/api/file", &root).is_none());
        // a board from before the rename (`.diagram`) is one too
        assert_eq!(super::boards(&root), ["a.shape", "sub/b.diagram"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decodes_query() {
        assert_eq!(
            super::query_param("/api/git?ref=HEAD%7E1&x=1", "ref").as_deref(),
            Some("HEAD~1")
        );
    }
}
