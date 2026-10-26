//! The editor in a native window (`shape FILE`), and the save dialog that
//! names a new board (`shape` with no FILE).
//!
//! The window is a system webview (WKWebView on macOS, WebView2 on Windows,
//! WebKitGTK on Linux) pointed at the local server, which keeps running on a
//! thread beside it; closing the window saves what is pending and ends the
//! process. Built without the `window` feature, or where no window can be
//! made (no display), boards open in the browser as before.
use std::path::{Path, PathBuf};

/// Where the editor opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    Window,
    Browser,
    Nowhere,
}

/// `--no-open` wins, then `--browser`; otherwise a window when one is built in.
pub fn choose(no_open: bool, browser: bool, window_built: bool) -> Open {
    if no_open {
        Open::Nowhere
    } else if browser || !window_built {
        Open::Browser
    } else {
        Open::Window
    }
}

/// The board's file name, for the window title.
pub fn board_name(file: &Path) -> String {
    file.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.display().to_string())
}

/// The window title for the page's title (`plan.shape — Shape`).
#[cfg_attr(not(feature = "window"), allow(dead_code))]
pub fn title(page: &str) -> String {
    let name = page.trim().trim_end_matches("— Shape").trim();
    if name.is_empty() || name == "Shape" {
        "Shape".into()
    } else {
        format!("{name} — Shape")
    }
}

/// Where the new-board dialog starts: from a terminal, the current
/// directory; started from the OS, Documents (else home, else here).
pub fn start_dir(from_terminal: bool) -> PathBuf {
    let here = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if from_terminal {
        return here;
    }
    let home = crate::home_dir();
    match home {
        Some(h) if h.join("Documents").is_dir() => h.join("Documents"),
        Some(h) if h.is_dir() => h,
        _ => here,
    }
}

/// A board in a process (and window) of its own, as from a terminal.
#[cfg_attr(not(feature = "window"), allow(dead_code))]
pub fn open_elsewhere(file: &Path) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = crate::command(exe)
            .arg(file)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

/// A link the board points out of the editor: to the system browser, and
/// only web and mail links (a board from someone else's branch could hold any).
#[cfg_attr(not(feature = "window"), allow(dead_code))]
pub fn open_external(url: &str) {
    let lower = url.to_ascii_lowercase();
    if !["http://", "https://", "mailto:"]
        .iter()
        .any(|s| lower.starts_with(s))
    {
        eprintln!("shape: not opening {url}");
        return;
    }
    let mut cmd = if cfg!(target_os = "macos") {
        crate::command("open")
    } else if cfg!(windows) {
        // not `cmd /C start`: cmd would read the & of a query string
        let mut c = crate::command("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    } else {
        crate::command("xdg-open")
    };
    let _ = cmd
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(feature = "window")]
pub use native::{pick_folder, Native};

#[cfg(not(feature = "window"))]
pub use none::{pick_folder, Native};

#[cfg(not(feature = "window"))]
mod none {
    use std::path::{Path, PathBuf};

    /// Built without the `window` feature: there is never a window.
    pub enum Native {}

    /// No window: no folder picker (None).
    pub fn pick_folder(_dir: &Path) -> Option<Option<PathBuf>> {
        None
    }

    impl Native {
        pub fn init() -> Option<Native> {
            None
        }
        pub fn run(self, _url: &str, _name: &str) -> Result<(), String> {
            match self {}
        }
        pub fn hand_over(self, _fallback: impl FnOnce() -> Option<PathBuf>) -> ! {
            match self {}
        }
    }
}

#[cfg(feature = "window")]
mod native {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use tao::dpi::{LogicalPosition, LogicalSize};
    use tao::event::{Event, StartCause, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy};
    use tao::window::WindowBuilder;
    use wry::{NewWindowResponse, WebViewBuilder};

    enum Ev {
        /// the window's close button, ⌘Q, ⌘W: save, then close
        CloseRequested,
        /// saved (or given up waiting): go
        Close,
        /// no answer from the page in time: go, unless it is asking something
        CloseTimeout,
        /// the page is asking the person something before it closes (where
        /// to save a new board): wait for it
        Wait,
        /// they chose to keep working: do not close
        Resume,
        /// the native folder picker, from `dir`; the answer goes back
        Pick(PathBuf, std::sync::mpsc::Sender<Option<PathBuf>>),
        Title(String),
        /// what Check for Updates… found: the newer version, or why it could not tell
        #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
        Updates(Result<Option<crate::update::Update>, String>),
        /// Shape → Install the shape Command… (in Shape.app)
        #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
        InstallCommand,
    }

    /// Saves what the editor has pending, then tells us to close.
    const FLUSH: &str = "Promise.resolve(window.diagramFlush ? window.diagramFlush() : null)\
        .catch(function () {})\
        .then(function () { window.ipc.postMessage('close'); });";

    /// An event loop made before anything else touches the windowing
    /// system (on macOS it must own the application object, which the
    /// save dialog would otherwise create).
    pub struct Native {
        event_loop: EventLoop<Ev>,
    }

    /// The window's events, for the server's requests (a folder picker).
    static PROXY: std::sync::OnceLock<std::sync::Mutex<EventLoopProxy<Ev>>> =
        std::sync::OnceLock::new();

    /// The native folder picker, opened over the window from `dir`: Some(None)
    /// when cancelled, None when there is no window to open it from.
    pub fn pick_folder(dir: &Path) -> Option<Option<PathBuf>> {
        let (tx, rx) = std::sync::mpsc::channel();
        PROXY
            .get()?
            .lock()
            .ok()?
            .send_event(Ev::Pick(dir.to_path_buf(), tx))
            .ok()?;
        rx.recv().ok()
    }

    impl Native {
        /// None where no window can be made (no display on Linux).
        pub fn init() -> Option<Native> {
            if cfg!(all(unix, not(target_os = "macos")))
                && std::env::var_os("DISPLAY").is_none()
                && std::env::var_os("WAYLAND_DISPLAY").is_none()
            {
                return None;
            }
            #[cfg(windows)]
            hide_console();
            let made =
                std::panic::catch_unwind(|| EventLoopBuilder::<Ev>::with_user_event().build());
            made.ok().map(|event_loop| Native { event_loop })
        }

        /// Shape.app opened from Finder or the Dock: the boards it was asked
        /// to open come as events once it has started, not as arguments.
        /// Each goes to a process (and window) of its own; none (the app
        /// itself was opened): `fallback`, a new board. Then this one ends.
        pub fn hand_over(self, fallback: impl FnOnce() -> Option<PathBuf> + 'static) -> ! {
            let until = std::time::Instant::now() + Duration::from_millis(800);
            let mut fallback = Some(fallback);
            self.event_loop.run(move |event, _, flow| {
                *flow = ControlFlow::WaitUntil(until);
                match event {
                    Event::Opened { urls } => {
                        for f in urls.iter().filter_map(|u| u.to_file_path().ok()) {
                            super::open_elsewhere(&f);
                        }
                        *flow = ControlFlow::Exit;
                    }
                    Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                        if let Some(f) = fallback.take().and_then(|f| f()) {
                            super::open_elsewhere(&f);
                        }
                        *flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            })
        }

        /// Show the editor at `url` until the window closes, then end the
        /// process. Returns only when the window could not be made.
        pub fn run(self, url: &str, name: &str) -> Result<(), String> {
            let event_loop = self.event_loop;
            let proxy = event_loop.create_proxy();
            let _ = PROXY.set(std::sync::Mutex::new(event_loop.create_proxy()));

            // 1400×900, smaller on a small screen, centred
            let (mut size, mut at) = (LogicalSize::<f64>::new(1400.0, 900.0), None);
            if let Some(m) = event_loop.primary_monitor() {
                let f = m.scale_factor();
                let s = m.size().to_logical::<f64>(f);
                let p = m.position().to_logical::<f64>(f);
                size = LogicalSize::new(
                    size.width.min(s.width * 0.92),
                    size.height.min(s.height * 0.9),
                );
                at = Some(LogicalPosition::new(
                    p.x + (s.width - size.width) / 2.0,
                    p.y + (s.height - size.height) / 2.0,
                ));
            }
            let mut builder = WindowBuilder::new()
                .with_title(super::title(name))
                .with_inner_size(size)
                .with_window_icon(icon());
            if let Some(at) = at {
                builder = builder.with_position(at);
            }
            let window = builder.build(&event_loop).map_err(|e| e.to_string())?;

            let origin = url.trim_end_matches('/').to_string();
            let (p_ipc, p_title) = (proxy.clone(), proxy.clone());
            let webview = WebViewBuilder::new()
                .with_url(url)
                .with_clipboard(true)
                // the editor's own pages stay; links out go to the browser
                .with_navigation_handler(move |u: String| {
                    let inside = u == origin
                        || u.starts_with(&format!("{origin}/"))
                        || u.starts_with(&format!("{origin}?"))
                        || ["about:", "blob:", "data:"]
                            .iter()
                            .any(|s| u.starts_with(s));
                    if !inside {
                        super::open_external(&u);
                    }
                    inside
                })
                .with_new_window_req_handler(|u: String, _| {
                    super::open_external(&u);
                    NewWindowResponse::Deny
                })
                .with_download_started_handler(|_, path: &mut PathBuf| {
                    eprintln!("shape: saving {}", path.display());
                    true
                })
                .with_ipc_handler(move |req| {
                    let ev = match req.body().as_str() {
                        "close" => Ev::Close,
                        "wait" => Ev::Wait,
                        "resume" => Ev::Resume,
                        _ => return,
                    };
                    let _ = p_ipc.send_event(ev);
                })
                .with_document_title_changed_handler(move |t| {
                    let _ = p_title.send_event(Ev::Title(t));
                });
            #[cfg(any(target_os = "macos", windows))]
            let webview = webview.build(&window);
            #[cfg(not(any(target_os = "macos", windows)))]
            let webview = {
                use tao::platform::unix::WindowExtUnix;
                use wry::WebViewBuilderExtUnix;
                match window.default_vbox() {
                    Some(vbox) => webview.build_gtk(vbox),
                    None => return Err("no GTK container for the webview".into()),
                }
            };
            let webview = webview.map_err(|e| e.to_string())?;

            #[cfg(target_os = "macos")]
            mac::name();
            #[cfg(target_os = "macos")]
            let menu = mac::menu(proxy.clone());
            let (mut closing, mut waiting) = (false, false);
            event_loop.run(move |event, _, flow| {
                *flow = ControlFlow::Wait;
                match event {
                    Event::NewEvents(StartCause::Init) => {
                        #[cfg(target_os = "macos")]
                        {
                            menu.init_for_nsapp();
                            mac::dock_icon();
                        }
                    }
                    // more boards from Finder: each in a window of its own
                    Event::Opened { urls } => {
                        for f in urls.iter().filter_map(|u| u.to_file_path().ok()) {
                            super::open_elsewhere(&f);
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::CloseRequested,
                        ..
                    }
                    | Event::UserEvent(Ev::CloseRequested) => {
                        if !closing {
                            closing = true;
                            crate::log::event("stop", serde_json::json!({ "why": "the window was closed" }));
                            // a page that cannot answer must not keep the window open
                            if webview.evaluate_script(FLUSH).is_err() {
                                *flow = ControlFlow::Exit;
                            }
                            let p = proxy.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(Duration::from_secs(3));
                                let _ = p.send_event(Ev::CloseTimeout);
                            });
                        }
                    }
                    Event::UserEvent(Ev::Close) => *flow = ControlFlow::Exit,
                    Event::UserEvent(Ev::CloseTimeout) if !waiting => *flow = ControlFlow::Exit,
                    Event::UserEvent(Ev::Wait) => waiting = true,
                    Event::UserEvent(Ev::Resume) => {
                        (closing, waiting) = (false, false);
                    }
                    Event::UserEvent(Ev::Pick(dir, reply)) => {
                        let picked = rfd::FileDialog::new()
                            .set_title("Where new boards go")
                            .set_directory(&dir)
                            .set_parent(&window)
                            .pick_folder();
                        let _ = reply.send(picked);
                    }
                    Event::UserEvent(Ev::Title(t)) => window.set_title(&super::title(&t)),
                    Event::UserEvent(Ev::Updates(found)) => tell_updates(found),
                    #[cfg(target_os = "macos")]
                    Event::UserEvent(Ev::InstallCommand) => mac::install_command(),
                    _ => {}
                }
            })
        }
    }

    /// Opened from Explorer or the Start menu, shape.exe (a console program)
    /// gets a console window of its own: with a window of Shape's coming, it goes.
    /// A terminal it was started from stays.
    #[cfg(windows)]
    fn hide_console() {
        extern "system" {
            fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
            fn FreeConsole() -> i32;
        }
        let mut ids = [0u32; 2];
        // SAFETY: plain kernel32 calls on a buffer of the size given
        unsafe {
            if GetConsoleProcessList(ids.as_mut_ptr(), 2) == 1 {
                FreeConsole();
            }
        }
    }

    /// The app icon on the window (Windows, Linux; macOS takes a bundle's).
    fn icon() -> Option<tao::window::Icon> {
        if cfg!(target_os = "macos") {
            return None;
        }
        let svg = include_str!("../icons/app.svg");
        let (rgba, w, h) = crate::png::rgba(svg, 128.0 / 1024.0).ok()?;
        tao::window::Icon::from_rgba(rgba, w, h).ok()
    }

    /// macOS: text editing shortcuts reach a webview only through an Edit
    /// menu; Quit and Close go through the same save-first close as the
    /// window's button.
    /// What Check for Updates… found, in a dialog.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn tell_updates(found: Result<Option<crate::update::Update>, String>) {
        use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
        let current = env!("CARGO_PKG_VERSION");
        match found {
            Ok(Some(u)) => {
                let how = if u.how.starts_with("http") {
                    format!("Download it from {}", u.how)
                } else {
                    format!("To upgrade, run `{}` in a terminal, then open your board again.", u.how)
                };
                let pick = MessageDialog::new()
                    .set_level(MessageLevel::Info)
                    .set_title(format!("Shape {} is out", u.latest))
                    .set_description(format!("You have {current}. {how}"))
                    .set_buttons(MessageButtons::OkCancelCustom("See what's new".into(), "Later".into()))
                    .show();
                if matches!(pick, MessageDialogResult::Custom(ref b) if b == "See what's new") {
                    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(windows) { "explorer" } else { "xdg-open" };
                    let _ = crate::command(opener).arg(&u.url).spawn();
                }
            }
            Ok(None) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Info)
                    .set_title("Shape is up to date")
                    .set_description(format!("You have the latest version, {current}."))
                    .set_buttons(MessageButtons::Ok)
                    .show();
            }
            Err(e) => {
                MessageDialog::new()
                    .set_level(MessageLevel::Warning)
                    .set_title("Could not check for updates")
                    .set_description(e)
                    .set_buttons(MessageButtons::Ok)
                    .show();
            }
        }
    }

    #[cfg(target_os = "macos")]
    mod mac {
        use muda::accelerator::{Accelerator, Code, Modifiers};
        use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem as P, Submenu};

        use super::{Ev, EventLoopProxy};

        /// Shape's icon and name in the Dock: the binary runs outside an app
        /// bundle, so macOS would show a plain program icon (and "exec").
        pub fn dock_icon() {
            use objc2_app_kit::NSApplication;
            use objc2_foundation::MainThreadMarker;
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let Some(image) = icon() else {
                return;
            };
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.setApplicationIconImage(Some(&image)) };
        }

        /// The app's name, "Shape", in the menu bar and the Dock (the process
        /// is called `shape`). Set before the menu is made, or the menu bar
        /// keeps the old one.
        pub fn name() {
            use objc2_foundation::{NSProcessInfo, NSString};
            NSProcessInfo::processInfo().setProcessName(&NSString::from_str("Shape"));
        }

        fn icon() -> Option<objc2::rc::Retained<objc2_app_kit::NSImage>> {
            use objc2::AnyThread;
            use objc2_app_kit::NSImage;
            use objc2_foundation::NSData;
            let png = crate::png::render(include_str!("../icons/app.svg"), 0.5).ok()?;
            NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&png))
        }

        /// Shape → Install the shape Command…: what happened, in a dialog.
        pub fn install_command() {
            use rfd::{MessageButtons, MessageDialog, MessageLevel};
            let Some(app) = crate::app::bundle() else {
                return;
            };
            let (level, title, text) = match crate::app::install_command(&app) {
                Ok(t) => (MessageLevel::Info, "The shape command", t),
                Err(e) => (MessageLevel::Warning, "Could not install the shape command", e),
            };
            MessageDialog::new()
                .set_level(level)
                .set_title(title)
                .set_description(text)
                .set_buttons(MessageButtons::Ok)
                .show();
        }

        /// Shape → About Shape: the standard panel, with the icon, the
        /// version, what Shape is and a link to the website.
        fn show_about() {
            use objc2::runtime::AnyObject;
            use objc2::AnyThread;
            use objc2_app_kit::{
                NSAboutPanelOptionApplicationIcon, NSAboutPanelOptionApplicationName,
                NSAboutPanelOptionApplicationVersion, NSAboutPanelOptionCredits,
                NSAboutPanelOptionVersion, NSApplication, NSAttributedStringAppKitDocumentFormats,
            };
            use objc2_foundation::{MainThreadMarker, NSAttributedString, NSData, NSDictionary, NSString};
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let html = format!(
                "<div style=\"font: 11px -apple-system; text-align: center; color: {}\">{}<br><br><a href=\"{SITE}\">{}</a></div>",
                "#7f7f7f",
                "One board for the whole project: mindmaps, plans, wireframes, decisions and slides, in a text file in your repo. The AI you already pay for keeps it current.",
                SITE.trim_start_matches("https://").trim_end_matches('/'),
            );
            let data = NSData::with_bytes(html.as_bytes());
            let Some(credits) = (unsafe {
                NSAttributedString::initWithHTML_documentAttributes(NSAttributedString::alloc(), &data, None)
            }) else {
                return;
            };
            let name = NSString::from_str("Shape");
            let version = NSString::from_str(env!("CARGO_PKG_VERSION"));
            // the build number, under the version: the same, so the panel shows it once
            let build = NSString::from_str("");
            let mut keys: Vec<&NSString> = unsafe {
                vec![
                    NSAboutPanelOptionApplicationName,
                    NSAboutPanelOptionApplicationVersion,
                    NSAboutPanelOptionVersion,
                    NSAboutPanelOptionCredits,
                ]
            };
            let mut values: Vec<&AnyObject> = vec![name.as_ref(), version.as_ref(), build.as_ref(), credits.as_ref()];
            let image = icon();
            if let Some(i) = &image {
                keys.push(unsafe { NSAboutPanelOptionApplicationIcon });
                values.push(i.as_ref());
            }
            let options = NSDictionary::from_slices(&keys, &values);
            let app = NSApplication::sharedApplication(mtm);
            unsafe { app.orderFrontStandardAboutPanelWithOptions(&options) };
            app.activate();
        }

        const SITE: &str = "https://dobrinov.github.io/shape/";

        pub fn menu(proxy: EventLoopProxy<Ev>) -> Menu {
            let cmd = |key| Some(Accelerator::new(Modifiers::META, key));
            let quit = MenuItem::with_id("quit", "Quit Shape", true, cmd(Code::KeyQ));
            let about = MenuItem::with_id("about", "About Shape", true, None);
            let updates = MenuItem::with_id("updates", "Check for Updates…", true, None);
            let close = MenuItem::with_id("close", "Close Window", true, cmd(Code::KeyW));
            let command = MenuItem::with_id("command", "Install the shape Command…", true, None);
            let menu = Menu::new();
            let sep = P::separator;
            let app = Submenu::with_items("Shape", true, &[&about, &updates]);
            // in Shape.app: the terminal command comes from the app
            if let (Ok(app), Some(_)) = (&app, crate::app::bundle()) {
                let _ = app.append(&command);
            }
            let rest: [&dyn muda::IsMenuItem; 6] = [
                &sep(),
                &P::hide(None),
                &P::hide_others(None),
                &P::show_all(None),
                &sep(),
                &quit,
            ];
            if let Ok(app) = &app {
                let _ = app.append_items(&rest);
            }
            let edit = Submenu::with_items(
                "Edit",
                true,
                &[
                    &P::undo(None),
                    &P::redo(None),
                    &sep(),
                    &P::cut(None),
                    &P::copy(None),
                    &P::paste(None),
                    &P::select_all(None),
                ],
            );
            let window = Submenu::with_items(
                "Window",
                true,
                &[&P::minimize(None), &P::fullscreen(None), &sep(), &close],
            );
            for sub in [app, edit, window].into_iter().flatten() {
                let _ = menu.append(&sub);
            }
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                if e.id == "about" {
                    show_about();
                } else if e.id == "updates" {
                    // asking GitHub takes a moment: not on the window's thread
                    let proxy = proxy.clone();
                    std::thread::spawn(move || {
                        let _ = proxy.send_event(Ev::Updates(crate::update::check_now()));
                    });
                } else if e.id == "command" {
                    let _ = proxy.send_event(Ev::InstallCommand);
                } else if e.id == "quit" || e.id == "close" {
                    let _ = proxy.send_event(Ev::CloseRequested);
                }
            }));
            menu
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_pick_where_the_editor_opens() {
        assert_eq!(choose(false, false, true), Open::Window);
        assert_eq!(choose(false, false, false), Open::Browser);
        assert_eq!(choose(false, true, true), Open::Browser);
        assert_eq!(choose(true, false, true), Open::Nowhere);
        assert_eq!(choose(true, true, false), Open::Nowhere);
    }

    #[test]
    fn window_titles() {
        assert_eq!(title("plan.shape"), "plan.shape — Shape");
        assert_eq!(title("plan.shape — Shape"), "plan.shape — Shape");
        assert_eq!(title("Shape"), "Shape");
        assert_eq!(title(""), "Shape");
    }
}
