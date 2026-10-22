# 002: Media bridge · Design

## Overview

```
page (https://www.deezer.com)                         Rust
───────────────────────────────                       ─────────────────────────────────────────
navigator.mediaSession                                
  .setActionHandler(a, f) ──▶ handlers[a] = f         
  .metadata = …           ─┐                          
  .playbackState = …      ─┤ schedule(≤250 ms)        
  1 s check               ─┘   └─▶ invoke("report_media", {snapshot}) ──▶ bridge::report_media
                                                                          └─▶ MediaSnapshot::sanitize
                                                                              └─▶ Player::update ──▶ listeners
window.__deezerTauri.dispatch(name) ◀── webview.eval(action.script()) ◀── Player::send(Action)
```

The script is `src-tauri/scripts/bridge.js`, embedded with `include_str!` and registered with `initialization_script` (main frame only). It is a plain script wrapped in a function of `window`, with no build step and no test-only exports: `src-tauri/scripts/bridge.test.mjs` runs the same text in a `node:vm` context whose global is a fake window (a `MediaSession` with the engines' accessors, a fake clock and a fake `__TAURI_INTERNALS__.invoke`), the way an engine runs it.

## Snapshot contract

```json
{
  "title": "string",
  "artist": "string",
  "album": "string",
  "artwork": "https://… | null",
  "playbackState": "none | paused | playing",
  "actions": ["play", "pause", "nexttrack", "previoustrack", "…"]
}
```

Rust deserializes it into `MediaSnapshot` (camelCase), then `sanitize()` applies AC-7. The script also caps strings at 1024 characters before sending, so an odd page cannot make IPC messages grow without bound; Rust does not rely on that.

## Hooks on `metadata` and `playbackState`

The engines define both as accessors on `MediaSession.prototype`. The script defines an own accessor on the instance that forwards to the prototype's and schedules a snapshot. If the descriptor is missing or not configurable, the one-second check alone keeps the state current (AC-3).

## State in Rust

`media::Player` is `Arc<Mutex<PlayerState>>` plus a list of listeners. `update(snapshot)` returns a `Change` (`track_changed`, `playback_changed`, `actions_changed`) so listeners act only on what changed. Feature 003 uses `track_changed` for notifications. Listeners run outside the lock.

## IPC permission

`report_media` is an app command. `build.rs` declares it in the app manifest, so Tauri generates `allow-report-media`. The capability `capabilities/deezer-remote.json`:

```json
{
  "identifier": "deezer-remote",
  "windows": ["main"],
  "remote": { "urls": ["https://*.deezer.com"] },
  "permissions": ["allow-report-media"]
}
```

No plugin permission is listed for remote origins. The `default` capability (local origin) grants nothing, since the app has no local page.

## End-to-end harness (`e2e` feature)

Built only with `--features e2e`; release builds do not contain it.

- `DEEZER_TAURI_E2E_URL` replaces the start URL and its origin is added to `Policy` as `App`; a runtime capability grants `allow-report-media` to that origin.
- The webview is incognito, the window-state and single-instance plugins are not registered, and settings are read from `DEEZER_TAURI_E2E_SETTINGS`, so a run never touches the user's session or preferences.
- A driver thread waits for each expected state, sends the next action, and writes a JSON report to `DEEZER_TAURI_E2E_REPORT`; the process exits 0 when every step passed and 1 on the first failure or after 60 s.
- `tests/e2e.rs` serves `tests/fixtures/player.html` from a `TcpListener` on `127.0.0.1:0`, runs the binary (`CARGO_BIN_EXE_deezer-tauri`) and asserts on the exit status and the report.

## Security

- The page can only call `report_media`, and everything it sends is bounded and sanitized before it reaches a window title, a menu label or a notification (AC-7, AC-8, AC-9).
- Rust evaluates exactly five fixed scripts, one per `Action` (AC-10).
- `bridge.js` keeps its state in a closure; only `dispatch` and `version` are reachable from the page through `window.__deezerTauri`, which is non-writable and non-configurable once installed.

## Performance budget

- One IPC message per real change, plus at most one per second when the hooks missed something and a change happened; none when idle.
- The one-second check serializes about 200 bytes and compares strings: negligible next to the player.

## Alternatives considered

- Polling only: simpler, but a pause would take up to a second to reach the tray.
- Rust pulling state with `eval` and a callback: Tauri's `eval` returns no value, so it would need IPC anyway.
- Emitting a Tauri event from the page instead of a command: events from remote origins need the `core:event` permission, which would let the page listen to every app event.
