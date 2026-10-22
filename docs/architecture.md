# Architecture

Deezer Tauri is a desktop shell for the Deezer web player, written in Rust on Tauri 2. It replaces the community Electron ports with a native window around the operating system's own web engine, and adds what a browser tab lacks: a tray icon with playback controls, track notifications, a window that survives being closed, and a session that stays logged in.

It is not a Deezer client. It never talks to Deezer's APIs, never requests a stream and never touches audio: the web player does all of that, exactly as it does in a browser. See [ADR 0002](adr/0002-webview-shell-not-native-client.md).

## Engines

| Platform | Engine | Media integration provided by the engine |
| --- | --- | --- |
| Linux | WebKitGTK 4.1 | MPRIS (media keys, GNOME and KDE widgets) |
| Windows | WebView2 (Chromium) | System Media Transport Controls |
| macOS | WKWebView | Now Playing, as far as the engine exposes it (open question of feature 002) |

Measured on 2026-10-22 with WebKitGTK 2.52.6 and the default WebKitGTK user agent: the Deezer web player logs in, plays, reports `MediaSource.isTypeSupported` true for `audio/mpeg`, `audio/flac` and MP3 in MP4, sets `navigator.mediaSession.metadata` and `playbackState`, and registers the `play`, `pause`, `nexttrack`, `seekbackward`, `seekforward` and `seekto` action handlers; `previoustrack` appears only once there is a track to go back to (after the first skip in a queue). WebKitGTK publishes that session on MPRIS under the product name, and an MPRIS `Play` sent from outside resumed playback. This is why the shell does not ship its own media-key integration ([ADR 0003](adr/0003-media-session-bridge.md)).

## Components

```
┌──────────────────────────── main window (WebviewWindow "main") ───────────────────────────┐
│ https://www.deezer.com                                                                     │
│   bridge.js (initialization script, main frame only)                                       │
│     ├─ wraps navigator.mediaSession.setActionHandler   → remembers the player's handlers   │
│     ├─ observes metadata and playbackState              → invoke("report_media", snapshot) │
│     └─ window.__deezerTauri.dispatch(action)            ← webview.eval from Rust           │
└────────────────────────────────────────────────────────────────────────────────────────────┘
        │ IPC (capability "deezer-remote", remote origins https://*.deezer.com only)
        ▼
  media::Player (state)  ──▶ window title, tray menu model, notification on track change
  navigation::classify   ──▶ in-app, auth popup, system browser or blocked
  settings::Settings     ──▶ JSON file in the app config directory
  tray                   ──▶ menu events → media::Action → bridge dispatch
```

| Module | Responsibility | Pure (unit tested without a window) |
| --- | --- | --- |
| `navigation` | Classify every URL the page navigates to or opens | yes |
| `user_agent` | The user agent override per platform, if any | yes |
| `settings` | Load, default and save user preferences | yes (file IO through a path) |
| `media` | Snapshot validation, state transitions, title and notification text, action script | yes |
| `tray` | Menu model from the media state and settings | the model is; the tray itself is not |
| `bridge` | IPC command and dispatch into the webview | through Tauri's mock runtime |
| `shell` | Window creation, single instance, close-to-tray, wiring | end-to-end test only |

## Security

- The page is remote, so it is untrusted as far as the shell is concerned. Its only IPC surface is the `report_media` command, granted by a capability bound to `https://*.deezer.com`. No Tauri plugin command (file system, shell, opener, notification) is exposed to the page.
- Everything a snapshot carries is bounded and validated in Rust before it reaches the tray, the title or a notification ([feature 002](../specs/features/002-media-bridge/design.md#security)).
- Rust never evaluates text that came from the page. The only scripts it injects are a fixed initialization script and `dispatch` calls whose argument is one of a closed set of actions.
- Top-level navigation is limited to Deezer and the login providers it uses; any other link opens in the system browser, and non-web schemes are refused.
- The session lives in the engine's persistent data store under the app's data directory, never in the shell's own files.

## Data on disk

| Path | Content | Owner |
| --- | --- | --- |
| app config dir / `settings.json` | the shell's preferences | `settings` |
| app config dir / `.window-state.json` | window size and position | window-state plugin |
| app data dir | cookies, local storage and cache of the web player | the web engine |
| app log dir | rotating log | log plugin |

## Testing

| Level | Tool | What it proves |
| --- | --- | --- |
| Unit | `cargo test` | the pure modules above |
| Bridge | `node --test` | `bridge.js` against a fake `navigator.mediaSession`, with no browser |
| IPC | `cargo test` with Tauri's `test` feature | `report_media` updates the state and rejects malformed snapshots |
| End to end | `cargo test --features e2e` | the real binary, in the real engine, against a local fixture page that behaves like the player: the snapshot arrives, tray actions reach the page's handlers |

Every acceptance criterion is referenced by ID (`001 AC-3`) in the test that covers it. CI runs every level on Linux, Windows and macOS.

## Release

Tags `vX.Y.Z` build the installers on each platform and attach them to a draft GitHub release: `.deb`, `.rpm` and AppImage on Linux, NSIS and MSI on Windows, `.dmg` on macOS. The binaries are not code-signed (feature 004).
