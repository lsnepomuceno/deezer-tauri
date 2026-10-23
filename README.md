<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Deezer Tauri" width="112">
</p>

<h1 align="center">Deezer Tauri</h1>

<p align="center">
  The Deezer web player as a light desktop app for Linux, Windows and macOS.
  <br>Its own window on the system's web engine, a tray with the playback controls,
  <br>track notifications, and music that keeps playing when the window is closed.
</p>

<p align="center">
  <a href="https://github.com/lsnepomuceno/deezer-tauri/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/lsnepomuceno/deezer-tauri/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/lsnepomuceno/deezer-tauri/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/lsnepomuceno/deezer-tauri?color=a238ff"></a>
  <img alt="Platforms: Linux, Windows, macOS" src="https://img.shields.io/badge/platforms-Linux%20%C2%B7%20Windows%20%C2%B7%20macOS-5b12b8">
  <img alt="Rust and Tauri 2" src="https://img.shields.io/badge/Rust-Tauri%202-dea584">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue"></a>
</p>

> [!NOTE]
> Unofficial. Deezer Tauri is not made, endorsed or supported by Deezer. "Deezer" is a trademark of Deezer S.A., used here only to say what the app opens. You need your own Deezer account, and the player behaves exactly as it does on deezer.com.

## Why

On Linux there is no official Deezer app, and the community ports repackage the Windows build on an Electron that falls years behind (the one this project replaced ran Chromium 91 from 2021). On Windows and macOS the official app is a 230 MB Electron. Deezer Tauri is a native window around the web player on the web engine your system already keeps up to date: the `.deb` and the `.rpm` are under 3 MB.

## Features

- **The real web player**, logged in across restarts, on WebKitGTK (Linux), WebView2 (Windows) or WKWebView (macOS).
- **Tray menu** with the track that is playing, play or pause, next and previous, show or hide the window, and quit.
- **Keeps playing when you close the window** (a switch in the tray menu; on by default).
- **Track notifications** when a new track starts while the window is in the background (also a switch).
- **Media keys and system media controls** through the engine's own integration: MPRIS on Linux (GNOME and KDE media widgets, verified). On Windows and macOS this is whatever WebView2 and WKWebView provide; the tray works on all three either way.
- **One window**: launching the app again brings back the one that is running. It reopens where you left it.
- **Links that leave Deezer open in your browser**; sign-in with Google, Facebook or Apple stays in the app.
- **Window title follows the track**, so the taskbar and the window switcher show what is playing.
- Menu in **Portuguese or English**, from the system language.

## Install

Download the installer for your system from the [latest release](https://github.com/lsnepomuceno/deezer-tauri/releases/latest).

| System | File | Notes |
| --- | --- | --- |
| Ubuntu, Debian, Mint, Pop!_OS | `.deb` | `sudo apt install ./Deezer.Tauri_*_amd64.deb` also installs the GStreamer plugins the player needs |
| Fedora, openSUSE | `.rpm` | `sudo dnf install ./Deezer.Tauri-*.x86_64.rpm` |
| Any Linux | `.AppImage` | `chmod +x Deezer.Tauri_*_amd64.AppImage` and run it; about 145 MB, since it carries its own WebKitGTK and GStreamer |
| Windows 10 and 11 | `_x64-setup.exe` or `_x64_en-US.msi` | Installs WebView2 if it is missing |
| macOS 11 or later | `_aarch64.dmg` or `_x64.dmg` | `aarch64` for Apple Silicon, `x64` for Intel |

The installers are not code-signed, so the first launch asks for confirmation:

- **Windows**: SmartScreen says "Windows protected your PC". Choose *More info*, then *Run anyway*.
- **macOS**: Gatekeeper refuses to open it. Right-click the app in Applications, choose *Open*, then *Open* again; or run `xattr -dr com.apple.quarantine "/Applications/Deezer Tauri.app"`.
- **Linux on GNOME**: the tray icon needs the AppIndicator extension, which Ubuntu enables by default (elsewhere, install *AppIndicator and KStatusNotifierItem Support*).

## How it works

The app opens `https://www.deezer.com` in a window and adds what a browser tab lacks. It never talks to Deezer's APIs, never requests, decrypts or stores audio: the web player does all of that, as it does in a browser ([ADR 0002](docs/adr/0002-webview-shell-not-native-client.md)).

To drive the tray and the notifications, a small script reads the [Media Session](https://developer.mozilla.org/docs/Web/API/Media_Session_API) the player already publishes (title, artist, artwork, playing or paused) and calls the play, pause, next and previous handlers the player registered ([ADR 0003](docs/adr/0003-media-session-bridge.md)). The page can call exactly one command in the app, which only reports that state, and everything it sends is validated before it reaches a menu or a notification. Details in [`docs/architecture.md`](docs/architecture.md).

**Privacy**: no telemetry, no account of its own, no network access besides the pages you open. The session lives in the engine's data folder for the app, as a browser profile would.

## Building from source

You need [Rust](https://rustup.rs) (stable), Node 20 or newer, and on Linux:

```sh
# Debian and Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev \
  gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav \
  patchelf xvfb   # patchelf for the AppImage, xvfb to keep the end-to-end window off screen
```

Windows needs the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and macOS the Xcode command line tools, as for any [Tauri app](https://v2.tauri.app/start/prerequisites/).

```sh
make deps      # Node tools: the Tauri CLI and the icon renderer
make run       # the app in development
make check     # formatting, Clippy, bridge, unit, IPC and end-to-end tests
make bundle    # the installers for your platform, in src-tauri/target/release/bundle
make help      # every target
```

## Project

The project follows spec-driven development: each feature is specified in [`specs/features/`](specs/README.md) before it is built, every acceptance criterion has a test that names it, and CI runs every test on Linux, Windows and macOS. The end-to-end test launches the real app in each platform's engine against a local page that behaves like the player.

| Path | Content |
| --- | --- |
| `src-tauri/src/` | The app, in Rust. Decisions are pure modules (`navigation`, `media`, `tray`, `notify`, `settings`); `shell` wires them to Tauri |
| `src-tauri/scripts/bridge.js` | The Media Session bridge injected into the page, with its tests |
| `src-tauri/tests/` | IPC, capability and end-to-end tests, and the fixture page |
| `specs/`, `docs/` | Feature specs, architecture and decisions |

Contributions are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md). Security issues go through [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © Lucas Nepomuceno
