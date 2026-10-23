use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::webview::{NewWindowFeatures, NewWindowResponse, PageLoadEvent};
use tauri::{
    AppHandle, Manager, Runtime, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use crate::bridge::{self, MAIN_WINDOW};
use crate::i18n::{self, Labels};
use crate::media::{window_title, Change, MediaAction, MediaInfo, Player, PRODUCT_NAME};
use crate::navigation::{self, Destination, FrameDecision, LoadDecision, Policy, PopupDecision};
use crate::settings::{self, Settings};
use crate::tray::{self, MenuCommand};
use crate::user_agent::{self, Platform};

/// What the shell keeps between events, managed by Tauri.
pub struct Shell {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    labels: &'static Labels,
    last_app_url: Mutex<Url>,
    /// What the shell last asked of the window: the engine reports a hide or show late.
    window_visible: AtomicBool,
    applied_menu: Mutex<Vec<tray::MenuEntry>>,
}

impl Shell {
    fn settings(&self) -> Settings {
        *lock(&self.settings)
    }

    fn change_settings(&self, change: impl FnOnce(&mut Settings)) -> Settings {
        let mut settings = lock(&self.settings);
        change(&mut settings);
        if let Err(error) = settings.save(&self.settings_path) {
            log::warn!("could not save the settings: {error}");
        }
        *settings
    }
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(not(feature = "e2e"))]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all()
                        - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .with_filter(|label| label == MAIN_WINDOW)
                .build(),
        );
    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(bridge::invoke_handler())
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the application")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_window(_app);
            }
        });
}

/// Where the app starts and what it allows; the end-to-end build replaces it.
pub(crate) struct Launch {
    pub(crate) policy: Policy,
    pub(crate) start_url: Url,
    pub(crate) settings_path: PathBuf,
}

fn launch<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Launch> {
    let launch = Launch {
        policy: Policy::default(),
        start_url: navigation::START_URL.parse().expect("the start URL parses"),
        settings_path: app.path().app_config_dir()?.join(settings::FILE_NAME),
    };
    #[cfg(feature = "e2e")]
    let launch = crate::e2e::adapt(app, launch)?;
    Ok(launch)
}

fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    log::info!(
        "{PRODUCT_NAME} {} on {:?}",
        app.package_info().version,
        Platform::current()
    );
    let Launch {
        policy,
        start_url,
        settings_path,
    } = launch(app)?;

    let player = Arc::new(Player::default());
    app.manage(Arc::clone(&player));
    app.manage(Shell {
        settings: Mutex::new(Settings::load(&settings_path)),
        settings_path,
        labels: i18n::system_labels(),
        last_app_url: Mutex::new(start_url.clone()),
        window_visible: AtomicBool::new(true),
        applied_menu: Mutex::new(Vec::new()),
    });

    let window = build_main_window(app, policy, start_url)?;
    watch_close(app, &window);

    let handle = app.clone();
    player.subscribe(move |info, change| on_media_change(&handle, info, change));

    let model = menu_model(app);
    tray::install(app, &model, handle_command)?;
    *lock(&app.state::<Shell>().applied_menu) = model;

    #[cfg(feature = "e2e")]
    crate::e2e::start(app.clone(), player);
    Ok(())
}

fn build_main_window<R: Runtime>(
    app: &AppHandle<R>,
    policy: Policy,
    start_url: Url,
) -> tauri::Result<WebviewWindow<R>> {
    let frame_policy = policy.clone();
    let frame_app = app.clone();
    let popup_policy = policy.clone();
    let popup_app = app.clone();
    let load_app = app.clone();

    let builder = WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::External(start_url))
        .title(PRODUCT_NAME)
        .inner_size(1280.0, 800.0)
        .min_inner_size(480.0, 600.0)
        .center()
        .initialization_script(bridge::SCRIPT)
        // WebKitGTK gives a popup its opener's scripts, not its own: the close handling
        // has to be registered here too, and only acts in pages with an opener.
        .initialization_script(navigation::POPUP_SCRIPT)
        .on_navigation(move |url| match frame_policy.frame_navigation(url) {
            FrameDecision::Allow => true,
            FrameDecision::OpenExternally => {
                open_externally(&frame_app, url);
                false
            }
            FrameDecision::Refuse => {
                log::info!("refused navigation to {}", navigation::describe(url));
                false
            }
        })
        .on_new_window(move |url, features| match popup_policy.new_window(&url) {
            PopupDecision::OpenInApp => match open_popup(&popup_app, features) {
                Ok(window) => NewWindowResponse::Create { window },
                Err(error) => {
                    log::warn!(
                        "could not open a popup for {}: {error}",
                        navigation::describe(&url)
                    );
                    NewWindowResponse::Deny
                }
            },
            PopupDecision::OpenExternally => {
                open_externally(&popup_app, &url);
                NewWindowResponse::Deny
            }
            PopupDecision::Ignore => NewWindowResponse::Deny,
        })
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Started {
                return;
            }
            let url = payload.url();
            match policy.main_frame_load(url) {
                LoadDecision::LeaveToBrowser => {
                    open_externally(&load_app, url);
                    let back = lock(&load_app.state::<Shell>().last_app_url).clone();
                    if let Err(error) = window.navigate(back) {
                        log::warn!("could not return to the player: {error}");
                    }
                }
                LoadDecision::Stay => {
                    if policy.classify(url) == Destination::App
                        && matches!(url.scheme(), "http" | "https")
                    {
                        *lock(&load_app.state::<Shell>().last_app_url) = url.clone();
                    }
                }
            }
        });

    let builder = match user_agent::for_platform(Platform::current()) {
        Some(agent) => builder.user_agent(agent),
        None => builder,
    };
    #[cfg(feature = "e2e")]
    let builder = builder
        .incognito(true)
        .on_document_title_changed(|_, title| crate::e2e::record_title(&title));

    builder.build()
}

const POPUP_SIZE: (f64, f64) = (500.0, 650.0);

/// A popup the page opened on an allowed origin, such as a sign-in provider's (001 AC-8,
/// AC-11). The engine loads the URL into it; `window_features` relates it to the opener.
fn open_popup<R: Runtime>(
    app: &AppHandle<R>,
    features: NewWindowFeatures,
) -> tauri::Result<WebviewWindow<R>> {
    static NEXT_POPUP: AtomicUsize = AtomicUsize::new(1);
    let label = format!("popup-{}", NEXT_POPUP.fetch_add(1, Ordering::SeqCst));
    let closing_app = app.clone();
    let closing_label = label.clone();
    WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External("about:blank".parse().expect("about:blank parses")),
    )
    .title(PRODUCT_NAME)
    .inner_size(POPUP_SIZE.0, POPUP_SIZE.1)
    .center()
    .window_features(features)
    .initialization_script(navigation::POPUP_SCRIPT)
    .on_navigation(move |url| {
        if !navigation::is_close_request(url) {
            return true;
        }
        if let Some(popup) = closing_app.get_webview_window(&closing_label) {
            if let Err(error) = popup.close() {
                log::warn!("could not close the popup: {error}");
            }
        }
        false
    })
    .on_document_title_changed(|popup, title| {
        if let Err(error) = popup.set_title(&title) {
            log::warn!("could not set the popup title: {error}");
        }
    })
    .build()
}

fn watch_close<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let app = app.clone();
    let target = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            if app.state::<Shell>().settings().close_to_tray {
                api.prevent_close();
                hide_window(&app, &target);
            } else {
                app.exit(0);
            }
        }
    });
}

fn on_media_change<R: Runtime>(app: &AppHandle<R>, info: &MediaInfo, change: Change) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if change.track {
        if let Err(error) = window.set_title(&window_title(info)) {
            log::warn!("could not set the window title: {error}");
        }
    }
    refresh_tray(app);

    let focused = window.is_focused().unwrap_or(false);
    let settings = app.state::<Shell>().settings();
    if let Some(note) = crate::notify::decide(change, info, focused, &settings) {
        if let Err(error) = app
            .notification()
            .builder()
            .title(note.title)
            .body(note.body)
            .show()
        {
            log::warn!("could not show the notification: {error}");
        }
    }
}

pub fn handle_command<R: Runtime>(app: &AppHandle<R>, command: MenuCommand) {
    match command {
        MenuCommand::PlayPause => bridge::send(app, MediaAction::Toggle),
        MenuCommand::Next => bridge::send(app, MediaAction::Next),
        MenuCommand::Previous => bridge::send(app, MediaAction::Previous),
        MenuCommand::ToggleWindow => toggle_window(app),
        MenuCommand::CloseToTray => {
            app.state::<Shell>()
                .change_settings(|settings| settings.close_to_tray = !settings.close_to_tray);
            refresh_tray(app);
        }
        MenuCommand::Notifications => {
            app.state::<Shell>()
                .change_settings(|settings| settings.notifications = !settings.notifications);
            refresh_tray(app);
        }
        MenuCommand::Quit => app.exit(0),
    }
}

fn toggle_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let visible = app.state::<Shell>().window_visible.load(Ordering::SeqCst);
    if visible && !window.is_minimized().unwrap_or(false) {
        hide_window(app, &window);
    } else {
        show_window(app);
    }
}

fn hide_window<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    if let Err(error) = window.hide() {
        log::warn!("could not hide the window: {error}");
    }
    app.state::<Shell>()
        .window_visible
        .store(false, Ordering::SeqCst);
    refresh_tray(app);
}

fn show_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let shown = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(error) = shown {
        log::warn!("could not show the window: {error}");
    }
    app.state::<Shell>()
        .window_visible
        .store(true, Ordering::SeqCst);
    refresh_tray(app);
}

pub fn menu_model<R: Runtime>(app: &AppHandle<R>) -> Vec<tray::MenuEntry> {
    let shell = app.state::<Shell>();
    tray::menu_model(
        &app.state::<Arc<Player>>().current(),
        &shell.settings(),
        shell.window_visible.load(Ordering::SeqCst),
        shell.labels,
    )
}

/// The menu the tray shows now, as last applied.
pub fn applied_menu<R: Runtime>(app: &AppHandle<R>) -> Vec<tray::MenuEntry> {
    lock(&app.state::<Shell>().applied_menu).clone()
}

fn refresh_tray<R: Runtime>(app: &AppHandle<R>) {
    let model = menu_model(app);
    match tray::refresh(app, &model) {
        Ok(()) => *lock(&app.state::<Shell>().applied_menu) = model,
        Err(error) => log::warn!("could not refresh the tray: {error}"),
    }
}

fn open_externally<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    #[cfg(feature = "e2e")]
    if crate::e2e::intercept_external(url) {
        return;
    }
    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        log::warn!(
            "could not open {} in the system browser: {error}",
            navigation::describe(url)
        );
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
