//! 002 AC-7 and AC-8 through Tauri's IPC, with the mock runtime.

use std::sync::Arc;

use deezer_tauri::bridge;
use deezer_tauri::media::{PageAction, PlaybackState, Player};
use serde_json::{json, Value};
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindowBuilder};

fn invoke(body: Value) -> (Arc<Player>, Result<InvokeResponseBody, Value>) {
    let app = mock_builder()
        .manage(Arc::new(Player::default()))
        .invoke_handler(bridge::invoke_handler())
        .build(mock_context(noop_assets()))
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let response = get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: "report_media".into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    );
    let player = Arc::clone(app.state::<Arc<Player>>().inner());
    (player, response)
}

#[test]
fn ac_002_7_an_accepted_snapshot_reaches_the_player_sanitized() {
    let (player, response) = invoke(json!({
        "snapshot": {
            "title": "  Seize\nthe Day ",
            "artist": "Avenged Sevenfold",
            "album": "City of Evil",
            "artwork": "http://insecure.example/cover.jpg",
            "playbackState": "playing",
            "actions": ["play", "pause", "nexttrack", "rm -rf"]
        }
    }));
    assert!(response.is_ok(), "{response:?}");
    let info = player.current();
    assert_eq!(info.title, "Seize the Day");
    assert_eq!(info.artwork, None);
    assert_eq!(info.playback, PlaybackState::Playing);
    assert_eq!(
        info.actions.into_iter().collect::<Vec<_>>(),
        [PageAction::Play, PageAction::Pause, PageAction::NextTrack]
    );
}

#[test]
fn ac_002_8_a_malformed_snapshot_is_rejected_and_changes_nothing() {
    for body in [
        json!({ "snapshot": { "title": 42 } }),
        json!({ "snapshot": "Seize the Day" }),
        json!({ "snapshot": { "actions": "play" } }),
        json!({}),
    ] {
        let (player, response) = invoke(body.clone());
        assert!(response.is_err(), "{body} was accepted");
        assert_eq!(player.current(), Default::default(), "{body}");
    }
}
