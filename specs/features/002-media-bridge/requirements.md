# 002: Media bridge

## Problem

The shell needs to know what the player is playing, and to tell it to play, pause or skip, for the tray, the notifications and the window title (feature 003). It must do so without scraping the player's markup, which changes with every redesign, and without exposing anything on the user's machine to the remote page.

The player already publishes all of it through the Media Session API (`docs/architecture.md`, measured on 2026-10-22). The bridge reads that session and calls the handlers the player registered ([ADR 0003](../../../docs/adr/0003-media-session-bridge.md)).

## User stories

- As a listener, I want the window title to show the track that is playing, so that I can tell from the taskbar or the window switcher.
- As the tray and the notifications (feature 003), I want a current, validated view of the player's state and a way to send it an action.
- As a maintainer, I want the bridge to be tested without a browser and end to end in each real engine.

## Acceptance criteria

### Script (`bridge.js`)

- **AC-1:** Given the page loads, when the initialization script runs in the main frame before the player's code, then, unless the page is a popup (it has a `window.opener`: on Linux popups share the main webview's scripts), it installs `window.__deezerTauri` once (a second run does nothing) and, when `navigator.mediaSession` exists, wraps its `setActionHandler` so that every handler the page registers is remembered and still passed to the engine; registering `null` forgets the action.
- **AC-2:** Given an engine without `navigator.mediaSession`, when the script runs, then it installs a minimal one (`metadata`, `playbackState`, `setActionHandler`, `setPositionState`) and a `MediaMetadata` constructor, so the player feeds the bridge anyway.
- **AC-3:** Given the page sets `metadata` or `playbackState`, or registers or forgets a handler, when the change happens, then the script sends a snapshot `{title, artist, album, artwork, playbackState, actions}` to the `report_media` command within 250 ms; a snapshot equal to the last one sent is not sent again; a one-second check sends any change the property hooks missed.
- **AC-4:** Given the metadata lists several artwork images, when the snapshot is built, then `artwork` is the source of the largest by declared `sizes` (the last one when none declares a size), and `null` when there is none.
- **AC-5:** Given Tauri's IPC is not available yet when a snapshot is due, when it becomes available, then the latest snapshot is sent; a failed `invoke` is retried with the next change or check, never in a tight loop.
- **AC-6:** Given `dispatch(action)` is called, when `action` is one of `play`, `pause`, `nexttrack` or `previoustrack` and the page registered a handler for it, then that handler is called with `{action}` and `dispatch` returns `true`; `toggle` calls `pause` when `playbackState` is `playing` and `play` otherwise; any other action, or one without a handler, returns `false` and calls nothing; an exception thrown by the handler is caught and `dispatch` returns `false`.

### Rust side

- **AC-7:** Given a `report_media` call, when the snapshot is accepted, then the shared player state holds it and listeners (title, tray, notifications) are told; text fields are trimmed, stripped of control characters and cut at 300 characters on a character boundary; `artwork` is kept only when it is an `https` URL of at most 2048 bytes; `playbackState` other than `none`, `paused` or `playing` is read as `none`; unknown action names are dropped and at most 16 are kept.
- **AC-8:** Given a `report_media` call whose payload does not deserialize into a snapshot, when it arrives, then it is rejected with an error and the state is unchanged.
- **AC-9:** Given the capability configuration, when the page calls `report_media`, then it is allowed only from `https://*.deezer.com` in the `main` webview, and no plugin command is allowed from any remote origin.
- **AC-10:** Given an `Action` (`Toggle`, `Play`, `Pause`, `Next`, `Previous`), when it is sent to the player, then Rust evaluates a fixed script calling `window.__deezerTauri.dispatch` with the matching action name, and never a string that came from the page.
- **AC-11:** Given the state changes, when there is a title, then the window title becomes `<title> · <artist>` (the title alone when there is no artist), and `Deezer Tauri` when the title is empty.

### End to end

- **AC-12:** Given the binary built with the `e2e` feature and a local page that behaves like the player (sets metadata, registers `play`, `pause` and `nexttrack`, updates both when they are called), when the app loads it, then the snapshot reaches Rust, `Toggle` makes the page report `playing`, `Next` makes it report the second track, and the window title follows; this runs in WebKitGTK, WebView2 and WKWebView in CI.

## Out of scope

- Seeking, position and duration (the engine's own OS integration already offers them where it exists).
- Any OS media integration of our own (ADR 0003).
- Lyrics, queue, likes or anything the Media Session API does not carry.

## Open questions

- Decided on 2026-10-23: whether WKWebView publishes the player's session to macOS Now Playing for a non-Safari app is left unverified, since it needs a Mac with an account playing. It does not affect this feature (the tray works either way), and the README promises only what WebView2 and WKWebView provide on Windows and macOS.
