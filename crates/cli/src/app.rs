//! `shape install-app`: make .shape (and .diagram) files the OS's own, with a document
//! icon, opening in the editor on a double-click.
//!
//! - macOS: ~/Applications/Shape.app, a small AppleScript app (Finder hands
//!   files over as Apple Events, not arguments) that declares the board
//!   type (.shape, and .diagram from before the rename) and runs
//!   `shape FILE --idle-exit`. With the real Shape.app (Shape.dmg) in
//!   /Applications or ~/Applications, nothing is made: that app does it, and
//!   only our script copy is ever removed.
//! - Linux: a MIME type, its icon and a .desktop entry under ~/.local/share.
//! - Windows: a file association under HKEY_CURRENT_USER.
//!
//! A board opens in its own window and closing it stops the server; opening
//! the app itself asks where to save a new board. Where no window can be made
//! the board opens in a browser tab, and with no terminal to stop that server
//! it stops by itself once no editor tab has been heard from for a while.
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_SVG: &str = include_str!("../icons/app.svg");
const DOC_SVG: &str = include_str!("../icons/document.svg");

pub fn install(remove: bool) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        macos(remove)
    } else if cfg!(windows) {
        windows(remove)
    } else {
        linux(remove)
    }
}

/// The `shape` to start: the one on PATH (a stable path across Homebrew
/// upgrades), else this binary.
fn binary() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this binary: {e}"))?;
    let name = exe
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    let on_path = std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(&name))
            .find(|c| c.is_file() && same_file(c, &exe))
    });
    Ok(on_path.unwrap_or(exe))
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| "no home directory".to_string())
}

fn png(svg: &str, size: u32) -> Result<Vec<u8>, String> {
    crate::png::render(svg, size as f32 / 1024.0)
}

fn run(cmd: &mut Command) -> Result<(), String> {
    let what = format!("{:?}", cmd.get_program());
    let out = cmd.output().map_err(|e| format!("{what}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{what} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

// ------------------------------------------------------------------ macOS

const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
const UTI: &str = "io.github.dobrinov.shape.board";

/// An AppleScript string literal.
fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Whether files can be made in `dir` (made if missing).
fn writable(dir: &Path) -> bool {
    let probe = dir.join(".shape-probe");
    let ok = std::fs::create_dir_all(dir).is_ok() && std::fs::write(&probe, "").is_ok();
    let _ = std::fs::remove_file(probe);
    ok
}

fn macos(remove: bool) -> Result<(), String> {
    // the real Shape.app (from the DMG) declares the board type itself
    if let (Some(app), false) = (bundle(), remove) {
        remove_old_apps(&app);
        eprintln!("{} opens .shape and .diagram files.", app.display());
        return Ok(());
    }
    // the real app installed next to a `shape` from Homebrew: it does the job
    if let (Some(app), false) = (real_app(), remove) {
        remove_old_apps(&app);
        eprintln!("{} opens .shape and .diagram files.", app.display());
        return Ok(());
    }
    // ~/Applications when it is ours (on some Macs root owns it); any folder
    // works for Launch Services
    let home = home()?;
    let places = [
        home.join("Applications"),
        home.join("Library/Application Support/shape"),
    ];
    // Shape.app, and Diagram.app from before the rename
    let before = home.join("Library/Application Support/diagram");
    for old in places
        .iter()
        .chain([&before])
        .flat_map(|d| [d.join("Shape.app"), d.join("Diagram.app")])
    {
        // only our script copy: the real app (from Shape.dmg) stays
        if old.exists() && !script_app(&old) {
            if remove {
                eprintln!(
                    "left {} (the Shape app: move it to the Bin to remove it)",
                    old.display()
                );
            }
        } else if old.exists() {
            let _ = crate::command(LSREGISTER).arg("-u").arg(&old).output();
            std::fs::remove_dir_all(&old).map_err(|e| format!("{}: {e}", old.display()))?;
            if remove {
                eprintln!("removed {}", old.display());
            }
        }
    }
    if remove {
        return Ok(());
    }
    let app = places
        .iter()
        .find(|d| writable(d))
        .ok_or("cannot write to ~/Applications or ~/Library/Application Support")?
        .join("Shape.app");
    let bin = binary()?;
    // Finder starts apps with a bare PATH: keep this one, so the AI CLIs are found
    let path = std::env::var("PATH").unwrap_or_default();
    let script = format!(
        r#"on open theFiles
	repeat with f in theFiles
		do shell script "export PATH=" & quoted form of {path} & "; nohup " & quoted form of {bin} & " " & quoted form of (POSIX path of f) & " --idle-exit > /dev/null 2>&1 &"
	end repeat
end open

on run
	do shell script "export PATH=" & quoted form of {path} & "; nohup " & quoted form of {bin} & " --idle-exit > /dev/null 2>&1 &"
end run
"#,
        path = applescript_string(&path),
        bin = applescript_string(&bin.to_string_lossy()),
    );
    let tmp = std::env::temp_dir().join(format!("shape-app-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    write(&tmp.join("open.applescript"), script)?;
    run(crate::command("osacompile")
        .arg("-o")
        .arg(&app)
        .arg(tmp.join("open.applescript")))?;

    // the app's icon (replacing the script icon), the documents' and the board type
    let res = app.join("Contents").join("Resources");
    icons(&res, &tmp, "applet")?;
    let _ = std::fs::remove_file(res.join("Assets.car")); // would win over applet.icns
    let _ = std::fs::remove_dir_all(&tmp);
    let plist = app.join("Contents").join("Info.plist");
    // an app with an `open` handler is a "droplet" with its own icon: ours instead
    declare(&plist, "applet")?;
    let _ = std::fs::remove_file(res.join("droplet.icns"));
    run(crate::command(LSREGISTER).arg("-f").arg(&app))?;
    // Finder caches icons: a touch makes it look again
    let _ = crate::command("touch").arg(&app).output();
    eprintln!("installed {}", app.display());
    eprintln!(
        ".shape and .diagram files now have their own icon and open in Shape on a double-click."
    );
    eprintln!("If Finder still shows the old icon, run: killall Finder");
    Ok(())
}

/// `name.icns` (the app's) and `document.icns` in `res`, made in `tmp`.
fn icons(res: &Path, tmp: &Path, name: &str) -> Result<(), String> {
    std::fs::create_dir_all(res).map_err(|e| format!("{}: {e}", res.display()))?;
    for (svg, name) in [(APP_SVG, name), (DOC_SVG, "document")] {
        let set = tmp.join(format!("{name}.iconset"));
        for size in [16, 32, 128, 256, 512] {
            write(
                &set.join(format!("icon_{size}x{size}.png")),
                png(svg, size)?,
            )?;
            write(
                &set.join(format!("icon_{size}x{size}@2x.png")),
                png(svg, size * 2)?,
            )?;
        }
        run(crate::command("iconutil")
            .args(["-c", "icns", "-o"])
            .arg(res.join(format!("{name}.icns")))
            .arg(&set))?;
    }
    Ok(())
}

/// Shape's name, version and icon in an app's Info.plist, and the board
/// type: declared there, opened there, with its icon.
fn declare(plist: &Path, icon: &str) -> Result<(), String> {
    let set = |key: &str, json: String| {
        let _ = crate::command("plutil")
            .args(["-remove", key])
            .arg(plist)
            .output();
        run(crate::command("plutil")
            .args(["-insert", key, "-json", &json])
            .arg(plist))
    };
    set("CFBundleIdentifier", "\"io.github.dobrinov.shape\"".into())?;
    set("CFBundleName", "\"Shape\"".into())?;
    set("CFBundleIconFile", format!("\"{icon}\""))?;
    set(
        "CFBundleShortVersionString",
        format!("\"{}\"", env!("CARGO_PKG_VERSION")),
    )?;
    set(
        "UTExportedTypeDeclarations",
        format!(
            r#"[{{"UTTypeIdentifier":"{UTI}","UTTypeDescription":"Shape board","UTTypeConformsTo":["public.plain-text"],"UTTypeIconFile":"document.icns","UTTypeReferenceURL":"https://dobrinov.github.io/shape/","UTTypeTagSpecification":{{"public.filename-extension":["shape","diagram"]}}}}]"#
        ),
    )?;
    set(
        "CFBundleDocumentTypes",
        format!(
            r#"[{{"CFBundleTypeName":"Shape board","CFBundleTypeRole":"Editor","LSHandlerRank":"Owner","LSItemContentTypes":["{UTI}"],"CFBundleTypeIconFile":"document.icns"}}]"#
        ),
    )
}

/// `shape make-app OUT.app [--binary FILE]`: Shape.app, with the binary
/// (this one unless given) inside, for the DMG. It opens boards from Finder
/// itself (no script in between) and offers the `shape` command.
pub fn make_app(out: &Path, binary: Option<&Path>) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("make-app makes a macOS app: run it on a Mac".into());
    }
    let bin = match binary {
        Some(b) => b.to_path_buf(),
        None => std::env::current_exe().map_err(|e| format!("cannot find this binary: {e}"))?,
    };
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    }
    let contents = out.join("Contents");
    let exe = contents.join("MacOS").join("shape");
    write(
        &exe,
        std::fs::read(&bin).map_err(|e| format!("{}: {e}", bin.display()))?,
    )?;
    run(crate::command("chmod").arg("755").arg(&exe))?;
    let plist = contents.join("Info.plist");
    write(
        &plist,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleExecutable</key><string>shape</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
	<key>CFBundleDisplayName</key><string>Shape</string>
	<key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
	<key>LSMinimumSystemVersion</key><string>11.0</string>
	<key>NSHighResolutionCapable</key><true/>
	<key>NSHumanReadableCopyright</key><string>MIT licensed</string>
</dict>
</plist>
"#,
    )?;
    let tmp = std::env::temp_dir().join(format!("shape-make-app-{}", std::process::id()));
    icons(&contents.join("Resources"), &tmp, "Shape")?;
    let _ = std::fs::remove_dir_all(&tmp);
    declare(&plist, "Shape")?;
    run(crate::command("plutil")
        .args([
            "-insert",
            "CFBundleVersion",
            "-string",
            env!("CARGO_PKG_VERSION"),
        ])
        .arg(&plist))?;
    eprintln!("made {}", out.display());
    Ok(())
}

/// The Shape.app this binary runs from, if it does (the DMG's app).
pub fn bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let app = contents.parent()?;
    (cfg!(target_os = "macos")
        && macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && app.extension()? == "app")
        .then(|| app.to_path_buf())
}

/// Started from Finder or the Dock, an app gets a bare PATH: take the one
/// the person's login shell has, so git and the AI's CLI are found.
pub fn login_path() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let out = crate::command(&shell)
        .args(["-ilc", "printf '\\n__SHAPE_PATH__%s' \"$PATH\""])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    let mut path = out
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.rsplit_once("__SHAPE_PATH__")
                .map(|(_, p)| p.trim().to_string())
        })
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
    // where CLIs usually are, should the shell not say
    let home = std::env::var("HOME").unwrap_or_default();
    for d in [
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".into(),
        format!("{home}/.local/bin"),
        "/usr/bin".into(),
        "/bin".into(),
    ] {
        if !path.split(':').any(|p| p == d) {
            path.push(':');
            path.push_str(&d);
        }
    }
    std::env::set_var("PATH", path);
}

/// The Shape.app made by `install-app` before there was a real one: a
/// script calling `shape`. Running from the real app, they go, so Finder
/// opens boards in it (our own copy stays, wherever it was put).
pub fn remove_old_apps(keep: &Path) {
    let Ok(home) = home() else {
        return;
    };
    for d in [
        "Applications",
        "Library/Application Support/shape",
        "Library/Application Support/diagram",
    ] {
        for name in ["Shape.app", "Diagram.app"] {
            let old = home.join(d).join(name);
            if script_app(&old) && !same_file(&old, keep) {
                let _ = crate::command(LSREGISTER).arg("-u").arg(&old).output();
                let _ = std::fs::remove_dir_all(&old);
            }
        }
    }
    let _ = crate::command(LSREGISTER).arg("-f").arg(keep).output();
}

fn script_app(app: &Path) -> bool {
    app.join("Contents/Resources/Scripts/main.scpt").exists()
}

/// The real Shape.app (from Shape.dmg) in /Applications or ~/Applications.
fn real_app() -> Option<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    dirs.extend(home().ok().map(|h| h.join("Applications")));
    dirs.into_iter()
        .map(|d| d.join("Shape.app"))
        .find(|a| a.join("Contents/MacOS/shape").exists() && !script_app(a))
}

/// Shape → Install the shape Command…: `shape` (and `diagram`) in
/// /usr/local/bin, linked to the app, after asking for an admin password.
/// Leaves a `shape` installed some other way (Homebrew) alone.
#[cfg_attr(not(all(feature = "window", target_os = "macos")), allow(dead_code))]
pub fn install_command(app: &Path) -> Result<String, String> {
    let exe = app.join("Contents/MacOS/shape");
    let link = Path::new("/usr/local/bin/shape");
    if let Ok(to) = std::fs::read_link(link) {
        if same_file(&to, &exe) || to == exe {
            return Ok("The shape command is already installed.".into());
        }
    }
    // one from Homebrew or an installer, first on PATH: that one stays
    let other = std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join("shape"))
            .find(|c| c.is_file() && !same_file(c, &exe))
    });
    if let Some(other) = other.filter(|o| o != link) {
        return Err(format!(
            "A shape command is already installed at {} (by Homebrew or an installer). Remove it first to use the app's.",
            other.display()
        ));
    }
    if link.exists() {
        return Err(format!(
            "A shape command is already installed at {} (by Homebrew or an installer). Remove it first to use the app's.",
            link.display()
        ));
    }
    let q = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let e = q(&exe.to_string_lossy());
    let sh = format!(
        "mkdir -p /usr/local/bin && ln -sf {e} /usr/local/bin/shape && ln -sf {e} /usr/local/bin/diagram"
    );
    run(crate::command("osascript").args([
        "-e",
        &format!(
            "do shell script {} with administrator privileges",
            applescript_string(&sh)
        ),
    ]))?;
    Ok("Installed: run shape in a terminal (/usr/local/bin/shape).".into())
}

// ------------------------------------------------------------------ Linux

fn linux(remove: bool) -> Result<(), String> {
    let share = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or(home()?.join(".local").join("share"));
    let files = [
        share.join("mime/packages/shape.xml"),
        share.join("icons/hicolor/scalable/mimetypes/application-x-shape.svg"),
        share.join("icons/hicolor/scalable/apps/shape.svg"),
        share.join("applications/shape.desktop"),
    ];
    // and the files from before the rename to Shape
    let old = [
        share.join("mime/packages/diagram.xml"),
        share.join("icons/hicolor/scalable/mimetypes/application-x-diagram.svg"),
        share.join("icons/hicolor/scalable/apps/diagram.svg"),
        share.join("applications/diagram.desktop"),
    ];
    for f in files.iter().chain(&old) {
        let _ = std::fs::remove_file(f);
    }
    if !remove {
        let bin = binary()?;
        write(
            &files[0],
            r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-shape">
    <comment>Shape board</comment>
    <sub-class-of type="text/plain"/>
    <glob pattern="*.shape"/>
    <glob pattern="*.diagram"/>
  </mime-type>
</mime-info>
"#,
        )?;
        write(&files[1], DOC_SVG)?;
        write(&files[2], APP_SVG)?;
        write(
            &files[3],
            format!(
                "[Desktop Entry]\nType=Application\nName=Shape\nComment=One board for the whole project, with your AI\nExec=\"{}\" %f --idle-exit\nIcon=shape\nMimeType=application/x-shape;\nTerminal=false\nCategories=Office;Graphics;\n",
                bin.display()
            ),
        )?;
    }
    // refresh the caches when the tools are there
    let _ = crate::command("update-mime-database")
        .arg(share.join("mime"))
        .output();
    let _ = crate::command("update-desktop-database")
        .arg(share.join("applications"))
        .output();
    let _ = crate::command("gtk-update-icon-cache")
        .args(["-f", "-t"])
        .arg(share.join("icons/hicolor"))
        .output();
    if !remove {
        let _ = crate::command("xdg-mime")
            .args(["default", "shape.desktop", "application/x-shape"])
            .output();
    }
    eprintln!(
        "{} the .shape file type (and .diagram) under {}",
        if remove { "removed" } else { "installed" },
        share.display()
    );
    Ok(())
}

// ------------------------------------------------------------------ Windows

/// An .ico holding PNG images (Windows Vista and later read them).
fn ico(svg: &str) -> Result<Vec<u8>, String> {
    let sizes = [16u32, 32, 48, 256];
    let images: Vec<Vec<u8>> = sizes
        .iter()
        .map(|&s| png(svg, s))
        .collect::<Result<_, _>>()?;
    let mut out = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (size, img) in sizes.iter().zip(&images) {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend([dim, dim, 0, 0, 1, 0, 32, 0]);
        out.extend((img.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += img.len() as u32;
    }
    for img in images {
        out.extend(img);
    }
    Ok(out)
}

fn windows(remove: bool) -> Result<(), String> {
    let classes = r"HKCU\Software\Classes";
    let reg = |args: &[&str]| run(crate::command("reg").args(args));
    // the association from before the rename to Shape goes either way
    let _ = reg(&["delete", &format!(r"{classes}\diagram.board"), "/f"]);
    if remove {
        for ext in [".shape", ".diagram"] {
            let _ = reg(&["delete", &format!(r"{classes}\{ext}"), "/f"]);
        }
        let _ = reg(&["delete", &format!(r"{classes}\shape.board"), "/f"]);
        if let Some(lnk) = start_menu() {
            let _ = std::fs::remove_file(lnk);
        }
        eprintln!("removed the .shape and .diagram file associations and the Start menu entry");
        return Ok(());
    }
    let bin = binary()?;
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or(home()?)
        .join("shape");
    let icon = dir.join("document.ico");
    write(&icon, ico(DOC_SVG)?)?;
    let key = format!(r"{classes}\shape.board");
    for ext in [".shape", ".diagram"] {
        reg(&[
            "add",
            &format!(r"{classes}\{ext}"),
            "/ve",
            "/d",
            "shape.board",
            "/f",
        ])?;
    }
    reg(&["add", &key, "/ve", "/d", "Shape board", "/f"])?;
    reg(&[
        "add",
        &format!(r"{key}\DefaultIcon"),
        "/ve",
        "/d",
        &icon.to_string_lossy(),
        "/f",
    ])?;
    let open = format!("\"{}\" \"%1\" --idle-exit", bin.display());
    reg(&[
        "add",
        &format!(r"{key}\shell\open\command"),
        "/ve",
        "/d",
        &open,
        "/f",
    ])?;
    // Shape in the Start menu, with its icon: it asks where to save a new board
    if let Some(lnk) = start_menu() {
        let q = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "''"));
        let script = format!(
            "$s = (New-Object -ComObject WScript.Shell).CreateShortcut({lnk}); \
             $s.TargetPath = {bin}; $s.Arguments = '--idle-exit'; $s.IconLocation = {bin} + ',0'; \
             $s.WorkingDirectory = {home}; $s.Description = 'One board for the whole project, with your AI'; $s.Save()",
            lnk = q(&lnk),
            bin = q(&bin),
            home = q(&home()?),
        );
        run(crate::command("powershell").args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ]))?;
    }
    eprintln!(".shape and .diagram files now have their own icon and open in Shape, and Shape is in the Start menu (sign out and in if Explorer keeps the old icon)");
    Ok(())
}

/// The user's Start menu entry for Shape.
fn start_menu() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|d| PathBuf::from(d).join(r"Microsoft\Windows\Start Menu\Programs\Shape.lnk"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_render_and_pack() {
        let p = png(DOC_SVG, 32).unwrap();
        assert_eq!(&p[1..4], b"PNG");
        let i = ico(APP_SVG).unwrap();
        assert_eq!(&i[..6], &[0, 0, 1, 0, 4, 0]);
        // the first image starts where its directory entry says
        let at = u32::from_le_bytes(i[18..22].try_into().unwrap()) as usize;
        assert_eq!(&i[at + 1..at + 4], b"PNG");
        assert_eq!(applescript_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
    }

    /// icons/app.ico, built into shape.exe (build.rs), is app.svg's.
    /// `SHAPE_WRITE_ICONS=1 cargo test -p shape exe_icon` redraws it.
    #[test]
    fn the_exe_icon_is_the_apps() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("icons/app.ico");
        let made = ico(APP_SVG).unwrap();
        if std::env::var_os("SHAPE_WRITE_ICONS").is_some() {
            std::fs::write(&path, &made).unwrap();
        }
        let have = std::fs::read(&path).unwrap();
        assert!(
            have == made,
            "icons/app.ico is not app.svg's: SHAPE_WRITE_ICONS=1 cargo test -p shape exe_icon"
        );
    }
}
