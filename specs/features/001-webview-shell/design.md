# 001: Webview shell · Design

## Overview

One `WebviewWindow` labelled `main`, built in Rust at setup (not from `tauri.conf.json`, so the navigation and new-window hooks can be attached), pointing at `https://www.deezer.com/`. Three Tauri plugins do the generic work: `single-instance` (AC-4), `window-state` (AC-2) and `opener` (system browser, AC-8 to AC-10). The decisions live in two pure modules.

```
navigation::classify(&str) -> Destination { App, SignIn, Browser, Refuse }
navigation::allow_frame_navigation(&Url) -> FrameDecision { Allow, Refuse, OpenExternally }   // AC-7
navigation::new_window(&Url) -> PopupDecision { OpenInApp, OpenExternally, Ignore }            // AC-8
navigation::main_frame_load(&Url) -> LoadDecision { Stay, LeaveToBrowser }                     // AC-9
user_agent::for_platform(Platform) -> Option<&'static str>                                     // AC-5
```

`Policy` holds the host lists, so tests and the end-to-end build can extend `App` with a fixture origin without touching production rules.

## Why three hooks and not one

`on_navigation` receives only a URL. On WebKitGTK it is wired to `decide-policy` for `NavigationAction`, which WebKit raises for the main frame and for subframes; on WKWebView it is `decidePolicyForNavigationAction`, also raised for subframes. Refusing non-Deezer hosts there would break reCAPTCHA and the login providers' frames. So:

| Hook | Frames | Rule |
| --- | --- | --- |
| `on_navigation` | all | allow web schemes, refuse the rest, hand `mailto:` to the system (AC-7) |
| `on_new_window` | n/a (popups) | in-app for `App` and `SignIn`, system browser for `Browser` (AC-8) |
| `on_page_load` (Started) | main frame only | a `Browser` URL leaves to the system browser and the window returns to the last `App` URL (AC-9) |

Popups are created by the shell (`NewWindowResponse::Create`) with `window_features`, which gives them the opener's context (on Linux a webview related to the caller, on Windows the same environment, on macOS the same configuration: that is what keeps the session and `window.opener` for OAuth popups) and the size and position the page asked for. `NewWindowResponse::Allow` did that too, but on Linux wry's default window has no size: Deezer's Google sign-in opened in a window of about 200 × 200 pixels (found on 2026-10-23 by clicking the real button under Xvfb), and `window.close()` left it empty.

| Popup | Value |
| --- | --- |
| label | `popup-N`, a counter |
| size | the page's `width` and `height`, else 500 × 650 |
| title | the page's title, as it changes |
| initialization script | `window.close = () => location.href = "https://deezer-tauri.invalid/close"`, only in a page with a `window.opener`; registered on the popup and on the main window too, because on WebKitGTK a related webview runs its opener's scripts instead of its own (found when `window.close` stayed native in the popup) |
| navigation hook | `navigation::is_close_request` closes the window and refuses the navigation; every other URL is allowed (the provider's pages are its own business) |

## Data on disk

| Path | Owner | Notes |
| --- | --- | --- |
| app data dir | WebKitGTK `WebsiteDataManager`, WebView2 user data folder, WKWebsiteDataStore | session (AC-3); Tauri sets the directory from the bundle identifier |
| app config dir / `.window-state.json` | window-state plugin | AC-2 |

## Security

- The navigation rules are about where the user ends up, not about isolation: the page's only privilege is the capability of feature 002, scoped to `https://*.deezer.com`. A `SignIn` page or a popup has no IPC permission.
- URLs are logged with scheme and host only (AC-10); paths and query strings of login flows can carry tokens.

## Performance budget

- Cold start to window shown: under 1 s on the reference Linux machine (16 cores, NVMe); the player's own load time is Deezer's.
- Resident memory of the shell process (not the engine's web process): under 60 MB.
- Every navigation decision is a parse and a few string comparisons; no allocation beyond the URL parse.

## Alternatives considered

- Declaring the window in `tauri.conf.json`: no way to attach `on_navigation`, `on_new_window` or `on_page_load` to it.
- Injecting a script that intercepts link clicks and opens external ones through IPC: it would give the remote page a command that opens URLs on the user's machine. The page-load hook achieves the same without that surface.
- Overriding the user agent with Chrome's on Linux: unnecessary (the default worked on 2026-10-22) and it would make the player assume Chromium features WebKitGTK lacks.
