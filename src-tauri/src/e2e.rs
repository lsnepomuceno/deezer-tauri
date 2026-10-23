//! The end-to-end harness of specs/features/002-media-bridge (AC-12). Built only with
//! `--features e2e`: it loads a local fixture page instead of Deezer, drives it through
//! the same paths the tray uses, writes a report and exits. With
//! `DEEZER_TAURI_E2E_SMOKE=1` it loads Deezer itself and only checks that the page loads
//! and that the bridge reports through the production capability.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::ipc::CapabilityBuilder;
use tauri::{AppHandle, Manager, Runtime, Url};

use crate::bridge::MAIN_WINDOW;
use crate::i18n::system_labels;
use crate::media::{MediaInfo, PageAction, PlaybackState, Player};
use crate::shell::{self, Launch};
use crate::tray::{MenuCommand, MenuEntry};

const TIMEOUT: Duration = Duration::from_secs(100);
const ASK_AGAIN: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

struct Config {
    url: Url,
    settings: PathBuf,
    report: PathBuf,
    smoke: bool,
}

impl Config {
    fn from_env() -> Option<Self> {
        Some(Self {
            url: std::env::var("DEEZER_TAURI_E2E_URL").ok()?.parse().ok()?,
            settings: std::env::var_os("DEEZER_TAURI_E2E_SETTINGS")?.into(),
            report: std::env::var_os("DEEZER_TAURI_E2E_REPORT")?.into(),
            smoke: std::env::var("DEEZER_TAURI_E2E_SMOKE").is_ok_and(|value| value == "1"),
        })
    }
}

static ACTIVE: OnceLock<bool> = OnceLock::new();
static EXTERNAL: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Every title the main page had, in order: the page answers the smoke probes through
/// its title, and Deezer may rename the page right after.
static PAGE_TITLES: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// URLs the shell decided to open in an app popup.
static POPUPS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn active() -> bool {
    *ACTIVE.get_or_init(|| Config::from_env().is_some())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// During a run, URLs for the system browser are recorded instead of opened.
pub(crate) fn intercept_external(url: &Url) -> bool {
    if !active() {
        return false;
    }
    lock(&EXTERNAL).push(url.to_string());
    true
}

pub(crate) fn record_popup(url: &Url) {
    if active() {
        lock(&POPUPS).push(url.to_string());
    }
}

pub(crate) fn record_title(title: &str) {
    if active() {
        lock(&PAGE_TITLES).push(title.to_owned());
    }
}

fn answered(tag: &str) -> Option<serde_json::Value> {
    let prefix = format!("E2E:{tag} ");
    lock(&PAGE_TITLES)
        .iter()
        .rev()
        .find_map(|title| title.strip_prefix(&prefix))
        .and_then(|json| serde_json::from_str(json).ok())
}

/// Plays a public 30-second preview from Deezer's CDN, muted, inside the Deezer page,
/// and reports once it is two seconds in (or after 20 s, however far it got: a CI
/// runner without a sound card can be slow to start).
const PREVIEW_PROBE: &str = r#"(function () {
  var answer = function (value) { document.title = 'E2E:PREVIEW ' + JSON.stringify(value); };
  fetch('https://api.deezer.com/track/3135556').then(function (r) { return r.json(); }).then(function (track) {
    var audio = new Audio(track.preview);
    audio.muted = true;
    audio.play().catch(function (error) { answer({ error: String(error) }); });
    var started = Date.now();
    (function check() {
      if (audio.currentTime >= 2 || audio.error || Date.now() - started > 20000) {
        answer({ time: audio.currentTime, error: audio.error && audio.error.code });
      } else {
        setTimeout(check, 250);
      }
    })();
  }, function (error) { answer({ error: String(error) }); });
})();"#;

/// Finds the cookie banner's refuse button and the Google sign-in button on Deezer's
/// login page; with `click`, clicks them from the script (enough for WebView2), else
/// reports their centers for xdotool.
fn sign_in_probe(click: bool) -> String {
    format!(
        r#"(function () {{
  var click = {click};
  var label = function (e) {{ return ((e.innerText || '').trim() || e.getAttribute('aria-label') || '').trim(); }};
  var find = function (re) {{
    return Array.from(document.querySelectorAll('button, a, [role=button]')).find(function (e) {{
      return re.test(label(e)) && e.getBoundingClientRect().width > 0;
    }});
  }};
  var center = function (e) {{ var r = e.getBoundingClientRect(); return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }}; }};
  var tries = 0;
  (function look() {{
    var google = find(/^google$/i);
    if (!google && tries++ < 40) {{ setTimeout(look, 500); return; }}
    var consent = find(/^(recusar|reject|reject all|refuse|decline)$/i);
    if (click) {{
      if (consent) {{ consent.click(); }}
      setTimeout(function () {{ if (google) {{ google.click(); }} document.title = 'E2E:BUTTONS ' + JSON.stringify({{ consent: !!consent, google: !!google }}); }}, 1000);
    }} else {{
      document.title = 'E2E:BUTTONS ' + JSON.stringify({{ consent: consent ? center(consent) : null, google: google ? center(google) : null }});
    }}
  }})();
}})();"#
    )
}

pub(crate) fn adapt<R: Runtime>(app: &AppHandle<R>, launch: Launch) -> tauri::Result<Launch> {
    let Some(config) = Config::from_env() else {
        return Ok(launch);
    };
    if config.smoke {
        return Ok(Launch {
            policy: launch.policy,
            start_url: config.url,
            settings_path: config.settings,
        });
    }
    app.add_capability(
        CapabilityBuilder::new("e2e-fixture")
            .remote(format!("{}/*", config.url.origin().ascii_serialization()))
            .window(MAIN_WINDOW)
            .permission("allow-report-media"),
    )?;
    Ok(Launch {
        policy: launch.policy.with_app_origin(&config.url),
        start_url: config.url,
        settings_path: config.settings,
    })
}

#[derive(Serialize)]
struct Step {
    name: &'static str,
    ok: bool,
    detail: String,
}

pub(crate) fn start<R: Runtime>(app: AppHandle<R>, player: Arc<Player>) {
    let Some(config) = Config::from_env() else {
        return;
    };
    thread::spawn(move || {
        let driver = Driver::new(&app, &player, config.url.clone());
        let steps = if config.smoke {
            driver.run_smoke()
        } else {
            driver.run()
        };
        let passed = steps.iter().all(|step| step.ok);
        let report = serde_json::to_vec_pretty(&steps).unwrap_or_default();
        if let Err(error) = std::fs::write(&config.report, report) {
            log::error!("could not write the end-to-end report: {error}");
        }
        app.exit(if passed { 0 } else { 1 });
    });
}

struct Driver<'a, R: Runtime> {
    app: &'a AppHandle<R>,
    player: &'a Player,
    start_url: Url,
    deadline: Instant,
    steps: Vec<Step>,
    /// The main window's X id, found on the first click while its title is still the
    /// fixture track (the page renames it once a popup answers).
    main_window: Option<String>,
}

type Script<D> = Vec<fn(&mut D) -> bool>;

impl<'a, R: Runtime> Driver<'a, R> {
    fn new(app: &'a AppHandle<R>, player: &'a Player, start_url: Url) -> Self {
        Self {
            app,
            player,
            start_url,
            deadline: Instant::now() + TIMEOUT,
            steps: Vec::new(),
            main_window: None,
        }
    }

    fn run(self) -> Vec<Step> {
        let mut script: Script<Self> = vec![
            |d| {
                d.media("the fixture's snapshot reaches Rust", |info| {
                    info.title == "Fixture Track One"
                        && info.artist == "Fixture Artist"
                        && info.playback == PlaybackState::Paused
                        && info.can(PageAction::Play)
                        && info.can(PageAction::Pause)
                        && info.can(PageAction::NextTrack)
                })
            },
            |d| {
                d.media("the largest artwork is kept", |info| {
                    info.artwork.as_deref() == Some("https://e2e.invalid/cover-500.jpg")
                })
            },
            |d| d.window_title("Fixture Track One · Fixture Artist"),
            |d| {
                d.tray("the tray enables play and next, not previous", |model| {
                    enabled(model, MenuCommand::PlayPause) == Some(true)
                        && enabled(model, MenuCommand::Next) == Some(true)
                        && enabled(model, MenuCommand::Previous) == Some(false)
                })
            },
            |d| d.command(MenuCommand::PlayPause),
            |d| d.media("play-pause makes the page play", MediaInfo::is_playing),
            |d| d.command(MenuCommand::Next),
            |d| {
                d.media("next loads the second track", |info| {
                    info.title == "Fixture Track Two" && info.can(PageAction::PreviousTrack)
                })
            },
            |d| d.window_title("Fixture Track Two · Fixture Artist"),
            |d| {
                d.tray("the tray enables previous once it exists", |model| {
                    enabled(model, MenuCommand::Previous) == Some(true)
                })
            },
            |d| d.command(MenuCommand::Previous),
            |d| {
                d.media("previous goes back", |info| {
                    info.title == "Fixture Track One"
                })
            },
            |d| d.command(MenuCommand::PlayPause),
            |d| {
                d.media("play-pause pauses again", |info| {
                    info.playback == PlaybackState::Paused
                })
            },
            |d| d.command(MenuCommand::ToggleWindow),
            |d| d.window_visible("toggle hides the window", false),
            |d| {
                d.tray("the tray offers to show the hidden window", |model| {
                    label(model, MenuCommand::ToggleWindow) == Some(system_labels().show_window)
                })
            },
            |d| d.command(MenuCommand::ToggleWindow),
            |d| d.window_visible("toggle shows it again", true),
            |d| {
                d.tray("the tray offers to hide it again", |model| {
                    label(model, MenuCommand::ToggleWindow) == Some(system_labels().hide_window)
                })
            },
            // 001 AC-7: mailto goes to the system and the page stays.
            |d| d.eval("location.href = 'mailto:someone@example.org';"),
            |d| {
                d.external(
                    "mailto opens in the system handler",
                    "mailto:someone@example.org",
                )
            },
            |d| d.on_start_url("the player stays after mailto"),
            // 001 AC-9: a page outside Deezer leaves for the browser and the window returns.
            |d| d.eval("location.href = 'https://example.org/leave?code=secret';"),
            |d| {
                d.external(
                    "a page outside Deezer opens in the browser",
                    "https://example.org/leave?code=secret",
                )
            },
            |d| d.on_start_url("the window returns to the player"),
            |d| {
                d.media("the returned player reports again", |info| {
                    !info.title.is_empty()
                })
            },
        ];
        // 001 AC-8. WKWebView opens a popup only within a real user gesture, which the
        // macOS runner cannot produce; there the classification tests cover it alone.
        if cfg!(any(target_os = "linux", target_os = "windows")) {
            script.extend([
                (|d| d.open_popup("/popup.html")) as fn(&mut Self) -> bool,
                |d| {
                    d.media(
                        "a popup on an app origin opens in the app, with its opener",
                        |info| info.title.starts_with("Popup Opened "),
                    )
                },
                // The popup reports its own size: 520 × 640 as asked, or 500 × 650 where
                // the engine does not pass the request on (WebKitGTK).
                |d| {
                    d.media("the popup has a usable size", |info| {
                        matches!(
                            info.title.as_str(),
                            "Popup Opened 520x640" | "Popup Opened 500x650"
                        )
                    })
                },
                |d| d.eval("window.__popup && window.__popup.postMessage('close', '*');"),
                |d| {
                    d.ask(
                        "the popup's window.close() closes its window",
                        "document.title = 'E2E:POPUP ' + JSON.stringify({ closed: !!(window.__popup && window.__popup.closed) });",
                        "POPUP",
                        |answer| answer["closed"] == true,
                    )
                },
                |d| d.open_popup("https://example.org/popup"),
                |d| {
                    d.external(
                        "a popup outside Deezer opens in the browser",
                        "https://example.org/popup",
                    )
                },
            ]);
        }
        self.play(script)
    }

    fn run_smoke(self) -> Vec<Step> {
        let mut script: Script<Self> = vec![
            |d| {
                d.wait("deezer.com loads in the engine", || {
                    let titles = lock(&PAGE_TITLES).clone();
                    (
                        titles.iter().any(|title| title.contains("Deezer")),
                        format!("titles: {titles:?}"),
                    )
                })
            },
            |d| {
                let player = d.player;
                d.wait(
                    "the bridge reports from deezer.com through the production capability",
                    || {
                        let reports = player.reports();
                        (reports > 0, format!("reports: {reports}"))
                    },
                )
            },
            |d| {
                let url = d.current_url();
                d.wait("the window is still on Deezer", || {
                    let on_deezer = url
                        .as_ref()
                        .and_then(Url::host_str)
                        .is_some_and(|host| host.ends_with("deezer.com"));
                    (
                        on_deezer,
                        format!("url: {:?}", url.as_ref().map(Url::as_str)),
                    )
                })
            },
            |d| d.eval(PREVIEW_PROBE),
            |d| {
                d.wait("a Deezer preview plays in the engine", || {
                    match answered("PREVIEW") {
                        Some(answer) => {
                            let time = answer["time"].as_f64().unwrap_or(0.0);
                            (time >= 2.0, answer.to_string())
                        }
                        None => (false, "no answer yet".to_owned()),
                    }
                })
            },
        ];
        // 001 AC-8 and AC-11 against Deezer's real sign-in: the Google button must open
        // Google's page in an app popup, which must close like a sign-in popup does.
        if cfg!(any(target_os = "linux", target_os = "windows")) {
            script.extend([
                (|d| d.navigate("https://www.deezer.com/br/login")) as fn(&mut Self) -> bool,
                |d| {
                    let app = d.app;
                    d.wait("Deezer's login page loads", || {
                        let host = app
                            .get_webview_window(MAIN_WINDOW)
                            .and_then(|window| window.url().ok())
                            .and_then(|url| url.host_str().map(str::to_owned));
                        (
                            host.as_deref() == Some("account.deezer.com"),
                            format!("host: {host:?}"),
                        )
                    })
                },
                |d| d.press_google(),
                |d| {
                    d.wait("Google's sign-in opens in an app popup", || {
                        let opened = lock(&POPUPS).clone();
                        let google = opened.iter().any(|url| {
                            Url::parse(url)
                                .ok()
                                .is_some_and(|url| url.host_str() == Some("accounts.google.com"))
                        });
                        (google, format!("popups: {}", opened.len()))
                    })
                },
            ]);
        }
        // The popup is the shell's window on Linux, where closing it is the shell's job;
        // WebView2 closes its own popup windows.
        if cfg!(target_os = "linux") {
            script.extend([
                (|d| {
                    for popup in popups(d.app) {
                        let _ = popup.eval("window.close();");
                    }
                    true
                }) as fn(&mut Self) -> bool,
                |d| {
                    let app = d.app;
                    d.wait("Google's popup closes like a sign-in popup does", || {
                        let left = popups(app).len();
                        (left == 0, format!("popups left: {left}"))
                    })
                },
            ]);
        }
        self.play(script)
    }

    fn navigate(&mut self, url: &str) -> bool {
        let navigated = url.parse().ok().is_some_and(|url| {
            self.app
                .get_webview_window(MAIN_WINDOW)
                .is_some_and(|window| window.navigate(url).is_ok())
        });
        if !navigated {
            self.steps.push(Step {
                name: "navigate the main window",
                ok: false,
                detail: url.to_owned(),
            });
        }
        navigated
    }

    /// Refuses the cookie banner and presses "Google" on Deezer's login page: with real
    /// clicks on Linux (WebKitGTK needs a user gesture to open the popup), from the script
    /// on Windows.
    fn press_google(&mut self) -> bool {
        let real_clicks = cfg!(target_os = "linux");
        let found = self.ask(
            "the login page shows the Google button",
            &sign_in_probe(!real_clicks),
            "BUTTONS",
            |answer| answer["google"].as_bool() != Some(false) && !answer["google"].is_null(),
        );
        if !found || !real_clicks {
            return found;
        }
        let answer = answered("BUTTONS").unwrap_or_default();
        if let Some((x, y)) = point(&answer["consent"]) {
            if !self.click_at("Deezer Tauri", x, y) {
                return false;
            }
            thread::sleep(Duration::from_millis(1500));
            lock(&PAGE_TITLES).clear();
            let again = self.ask(
                "the Google button after the cookie banner",
                &sign_in_probe(false),
                "BUTTONS",
                |answer| !answer["google"].is_null(),
            );
            if !again {
                return false;
            }
        }
        let answer = answered("BUTTONS").unwrap_or_default();
        match point(&answer["google"]) {
            Some((x, y)) => self.click_at("Deezer Tauri", x, y),
            None => false,
        }
    }

    /// Evaluates a probe and waits for its answer in the page title, sending it again
    /// every few seconds: a probe sent while a document is being replaced dies with it.
    fn ask(
        &mut self,
        name: &'static str,
        probe: &str,
        tag: &str,
        accept: impl Fn(&serde_json::Value) -> bool,
    ) -> bool {
        let window = self.app.get_webview_window(MAIN_WINDOW);
        let mut sent_at: Option<Instant> = None;
        self.wait(name, || {
            if sent_at.is_none_or(|at| at.elapsed() >= ASK_AGAIN) {
                if let Some(window) = &window {
                    let _ = window.eval(probe);
                }
                sent_at = Some(Instant::now());
            }
            match answered(tag) {
                Some(answer) => (accept(&answer), answer.to_string()),
                None => (false, "no answer yet".to_owned()),
            }
        })
    }

    fn click_at(&mut self, title_prefix: &str, x: i64, y: i64) -> bool {
        let outcome = click_main_window(&mut self.main_window, title_prefix, x, y);
        let ok = outcome.is_ok();
        self.steps.push(Step {
            name: "click the page",
            ok,
            detail: outcome.err().unwrap_or_default(),
        });
        ok
    }

    fn play(mut self, script: Script<Self>) -> Vec<Step> {
        for step in script {
            if !step(&mut self) {
                break;
            }
        }
        self.steps
    }

    fn current_url(&self) -> Option<Url> {
        self.app
            .get_webview_window(MAIN_WINDOW)
            .and_then(|window| window.url().ok())
    }

    fn eval(&mut self, script: &str) -> bool {
        let sent = self
            .app
            .get_webview_window(MAIN_WINDOW)
            .is_some_and(|window| window.eval(script).is_ok());
        if !sent {
            self.steps.push(Step {
                name: "evaluate a script in the page",
                ok: false,
                detail: script.to_owned(),
            });
        }
        sent
    }

    fn external(&mut self, name: &'static str, expected: &'static str) -> bool {
        self.wait(name, || {
            let opened = lock(&EXTERNAL).clone();
            (
                opened.iter().any(|url| url == expected),
                format!("opened: {opened:?}"),
            )
        })
    }

    fn on_start_url(&mut self, name: &'static str) -> bool {
        let expected = self.start_url.clone();
        let app = self.app;
        // Long enough for a navigation that should not happen to show up.
        thread::sleep(Duration::from_millis(500));
        self.wait(name, || {
            let url = app
                .get_webview_window(MAIN_WINDOW)
                .and_then(|window| window.url().ok());
            (
                url.as_ref() == Some(&expected),
                format!("url: {:?}", url.as_ref().map(Url::as_str)),
            )
        })
    }

    /// Opens a popup the way a click would. WebKitGTK only opens one within a user
    /// gesture, so on Linux the page arms the URL and xdotool clicks the window.
    fn open_popup(&mut self, url: &str) -> bool {
        let target = match self.start_url.join(url) {
            Ok(target) => target,
            Err(error) => {
                self.steps.push(Step {
                    name: "build the popup URL",
                    ok: false,
                    detail: error.to_string(),
                });
                return false;
            }
        };
        if cfg!(target_os = "linux") {
            self.eval(&format!("window.__nextPopup = {:?};", target.as_str())) && self.click_page()
        } else {
            self.eval(&format!(
                "window.open({:?}, '_blank', 'width=520,height=640');",
                target.as_str()
            ))
        }
    }

    fn click_page(&mut self) -> bool {
        self.click_at("Fixture Track", 200, 200)
    }

    fn wait(&mut self, name: &'static str, mut probe: impl FnMut() -> (bool, String)) -> bool {
        loop {
            let (ok, detail) = probe();
            if ok || Instant::now() >= self.deadline {
                self.steps.push(Step { name, ok, detail });
                return ok;
            }
            thread::sleep(POLL);
        }
    }

    fn media(&mut self, name: &'static str, check: impl Fn(&MediaInfo) -> bool) -> bool {
        let player = self.player;
        self.wait(name, || {
            let info = player.current();
            (check(&info), format!("{info:?}"))
        })
    }

    fn window_title(&mut self, expected: &'static str) -> bool {
        let window = self.app.get_webview_window(MAIN_WINDOW);
        self.wait("the window title follows the track", || {
            let title = window
                .as_ref()
                .and_then(|window| window.title().ok())
                .unwrap_or_default();
            (title == expected, title)
        })
    }

    fn window_visible(&mut self, name: &'static str, expected: bool) -> bool {
        let window = self.app.get_webview_window(MAIN_WINDOW);
        self.wait(name, || {
            let visible = window
                .as_ref()
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(!expected);
            (visible == expected, format!("visible: {visible}"))
        })
    }

    fn tray(&mut self, name: &'static str, check: impl Fn(&[MenuEntry]) -> bool) -> bool {
        let app = self.app;
        let present = app.tray_by_id(crate::tray::TRAY_ID).is_some();
        self.wait(name, || {
            let model = shell::applied_menu(app);
            (
                present && check(&model),
                format!("tray: {present}, model: {model:?}"),
            )
        })
    }

    fn command(&mut self, command: MenuCommand) -> bool {
        shell::handle_command(self.app, command);
        true
    }
}

/// Clicks inside the main window with xdotool, the only way to give WebKitGTK a user
/// gesture from outside. The main window is this process's visible window whose title is
/// the fixture track; GTK also maps a 10 × 10 leader window, and popups have their own.
fn click_main_window(
    known: &mut Option<String>,
    title_prefix: &str,
    x: i64,
    y: i64,
) -> Result<(), String> {
    let window = match known {
        Some(window) => window.clone(),
        None => {
            let window = find_main_window(title_prefix)?;
            *known = Some(window.clone());
            window
        }
    };
    let clicked = std::process::Command::new("xdotool")
        .args([
            "mousemove",
            "--window",
            &window,
            &x.to_string(),
            &y.to_string(),
            "click",
            "1",
        ])
        .status()
        .map_err(|error| format!("xdotool: {error}"))?;
    if clicked.success() {
        Ok(())
    } else {
        Err(format!("xdotool click exited with {clicked}"))
    }
}

fn find_main_window(title_prefix: &str) -> Result<String, String> {
    let pid = std::process::id().to_string();
    let found = std::process::Command::new("xdotool")
        .args(["search", "--onlyvisible", "--pid", &pid])
        .output()
        .map_err(|error| format!("xdotool: {error}"))?;
    let mut seen = Vec::new();
    let mut main = None;
    for window in String::from_utf8_lossy(&found.stdout).lines() {
        let name = std::process::Command::new("xdotool")
            .args(["getwindowname", window])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
        if main.is_none() && name.starts_with(title_prefix) {
            main = Some(window.to_owned());
        }
        seen.push(format!("{window} {name:?}"));
    }
    main.ok_or_else(|| format!("no window titled {title_prefix:?} among {seen:?}"))
}

fn popups<R: Runtime>(app: &AppHandle<R>) -> Vec<tauri::WebviewWindow<R>> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("popup-"))
        .map(|(_, window)| window)
        .collect()
}

fn point(value: &serde_json::Value) -> Option<(i64, i64)> {
    Some((value["x"].as_i64()?, value["y"].as_i64()?))
}

fn label(model: &[MenuEntry], wanted: MenuCommand) -> Option<&'static str> {
    model.iter().find_map(|entry| match entry {
        MenuEntry::Item { command, text, .. } if *command == wanted => Some(*text),
        _ => None,
    })
}

fn enabled(model: &[MenuEntry], wanted: MenuCommand) -> Option<bool> {
    model.iter().find_map(|entry| match entry {
        MenuEntry::Item {
            command, enabled, ..
        } if *command == wanted => Some(*enabled),
        _ => None,
    })
}
