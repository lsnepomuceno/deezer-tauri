# 0003: Read and drive playback through the Media Session API

- Status: accepted
- Date: 2026-10-22

## Context

The tray, notifications and window title need to know what is playing and to send play, pause, next and previous to the player. The options were:

1. A native media library (such as `souvlaki`) registering the app with MPRIS, SMTC and Now Playing, fed by scraping the page.
2. Scraping the player's DOM: reading the title from elements and clicking its buttons.
3. The Media Session API the player already uses: `navigator.mediaSession.metadata`, `playbackState` and the action handlers it registers.

On 2026-10-22 WebKitGTK was observed publishing the player's media session on MPRIS by itself, with title, artist, album, artwork and length, and obeying an MPRIS `Play`. WebView2 does the same with SMTC, as Chromium does.

## Decision

Option 3. An initialization script wraps `setActionHandler` so it learns the handlers the player registers, reports metadata and playback state to Rust, and exposes `dispatch(action)`, which calls the player's own handler. The shell does not register with any OS media service.

## Consequences

- No duplicate entry in the system media controls: the engine's own integration is the only one, and the tray drives the same handlers it does.
- No DOM selectors, which change with every redesign of the player. The Media Session API is a web standard the player has a reason to keep, since browsers depend on it too.
- An action the player has not registered cannot be sent. The tray disables it instead of guessing (for example, `previoustrack` when there is no previous track).
- If an engine does not implement `navigator.mediaSession`, the script installs a minimal one, so the player still feeds the bridge; that engine then has no OS-level media integration, which is no worse than today.
