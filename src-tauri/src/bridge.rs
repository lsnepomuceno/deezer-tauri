use std::sync::Arc;

use tauri::ipc::Invoke;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::media::{MediaAction, MediaSnapshot, Player};

pub const SCRIPT: &str = include_str!("../scripts/bridge.js");

pub const MAIN_WINDOW: &str = "main";

/// The page's only IPC surface (capabilities/deezer-remote.json).
#[tauri::command]
pub fn report_media(snapshot: MediaSnapshot, player: State<'_, Arc<Player>>) {
    player.update(snapshot.sanitize());
}

/// The app's IPC handler, shared by the shell and the IPC tests.
pub fn invoke_handler<R: Runtime>() -> impl Fn(Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![report_media]
}

pub fn send<R: Runtime>(app: &AppHandle<R>, action: MediaAction) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if let Err(error) = window.eval(action.script()) {
        log::warn!("could not send {} to the player: {error}", action.name());
    }
}
