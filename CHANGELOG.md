# Changelog

Every release of Deezer Tauri. The reasoning behind a change lives in the feature specs (`specs/features/`) and in the decisions (`docs/adr/`).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-10-22

First release.

### Added

- The Deezer web player in its own window, on the operating system's web engine: WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS.
- A session that stays logged in across restarts, a window that reopens where it was left, and a second launch that brings back the running window instead of opening another.
- Links that leave Deezer open in the system browser; sign-in with Google, Facebook or Apple stays inside the app.
- A tray menu with the track that is playing, play or pause, next, previous, show or hide window, quit, and two switches: keep playing when the window is closed (on by default) and track notifications (on by default).
- A desktop notification when a new track starts while the window is in the background.
- The window title follows the track.
- Menu in Portuguese or English, from the system language.
- Media keys and the system's media controls through the engine's own integration (MPRIS on Linux).
- Installers: `.deb`, `.rpm` and AppImage on Linux, NSIS and MSI on Windows, `.dmg` for Apple Silicon and Intel on macOS.

[Unreleased]: https://github.com/lsnepomuceno/deezer-tauri/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lsnepomuceno/deezer-tauri/releases/tag/v0.1.0
