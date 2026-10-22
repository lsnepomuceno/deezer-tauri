//! The end-to-end harness of specs/features/002-media-bridge (AC-12). Built only with
//! `--features e2e`: it loads a local fixture page instead of Deezer, drives it through
//! the same paths the tray uses, writes a report and exits.

use std::path::PathBuf;
use std::sync::Arc;
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

const TIMEOUT: Duration = Duration::from_secs(60);
const POLL: Duration = Duration::from_millis(50);

struct Config {
    url: Url,
    settings: PathBuf,
    report: PathBuf,
}

impl Config {
    fn from_env() -> Option<Self> {
        Some(Self {
            url: std::env::var("DEEZER_TAURI_E2E_URL").ok()?.parse().ok()?,
            settings: std::env::var_os("DEEZER_TAURI_E2E_SETTINGS")?.into(),
            report: std::env::var_os("DEEZER_TAURI_E2E_REPORT")?.into(),
        })
    }
}

pub(crate) fn adapt<R: Runtime>(app: &AppHandle<R>, launch: Launch) -> tauri::Result<Launch> {
    let Some(config) = Config::from_env() else {
        return Ok(launch);
    };
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
        let steps = Driver::new(&app, &player).run();
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
    deadline: Instant,
    steps: Vec<Step>,
}

impl<'a, R: Runtime> Driver<'a, R> {
    fn new(app: &'a AppHandle<R>, player: &'a Player) -> Self {
        Self {
            app,
            player,
            deadline: Instant::now() + TIMEOUT,
            steps: Vec::new(),
        }
    }

    fn run(mut self) -> Vec<Step> {
        let script: &[fn(&mut Self) -> bool] = &[
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
        ];
        for step in script {
            if !step(&mut self) {
                break;
            }
        }
        self.steps
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
