# 001: Webview shell

## Problem

The community ports of the Deezer desktop app for Linux repackage the Windows build on an Electron that falls years behind: the one this project replaces ran Electron 13 (Chromium 91, May 2022), with no security fixes. Windows and macOS users have the official app, but it is a 230 MB Electron too. What a listener needs is the web player in its own window, light, kept current by the operating system's web engine, logged in across restarts, and well behaved as a desktop app.

## User stories

- As a listener, I want Deezer in its own window that opens where I left it, so that it feels like an app and not a browser tab.
- As a listener, I want to stay logged in after I close and reopen the app, so that I log in once.
- As a listener, I want launching the app again to bring back the window that is already open, so that I never get two players fighting over the audio.
- As a listener, I want links that leave Deezer (an artist's website, a share link) to open in my browser, so that the player window always shows the player.
- As a listener who logs in with Google, Facebook or Apple, I want that login to work inside the app.

## Acceptance criteria

- **AC-1:** Given the app starts, when the main window is created, then it loads `https://www.deezer.com/`, has the title `Deezer Tauri`, a default size of 1280 × 800 and a minimum of 480 × 600 logical pixels, and is centered when there is no saved position.
- **AC-2:** Given the window was moved, resized or maximized, when the app starts again, then the window comes back with the same size, position and maximized state.
- **AC-3:** Given the user logged in, when the app is closed and started again, then the player is still logged in (cookies and local storage persist in the engine's data store under the app data directory).
- **AC-4:** Given an instance is running, when the app is launched again, then no second window is created: the existing window is shown, unminimized and focused, and the second process exits.
- **AC-5:** Given the platform, when the main window is built, then the user agent is the engine's default on Linux and Windows, and Safari's on macOS (`Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15`), since WKWebView's default omits the `Version` and `Safari` tokens that browser detection expects.
- **AC-6:** Given a URL, when it is classified, then it is:
  - `App` for `http` and `https` on `deezer.com` or any of its subdomains, and for `about:blank` and `about:srcdoc`;
  - `SignIn` for `https` on `accounts.google.com`, `accounts.youtube.com`, `www.facebook.com`, `m.facebook.com`, `facebook.com` and `appleid.apple.com`;
  - `Browser` for any other `http` or `https` URL, and for `mailto:`;
  - `Refuse` for every other scheme (`file:`, `javascript:`, `data:`, custom schemes) and for URLs that do not parse.
  Host matching is exact or by subdomain on a label boundary (`evil-deezer.com` and `deezer.com.evil.net` are `Browser`), case-insensitive, and ignores the port.
- **AC-7:** Given a navigation in any frame, when the engine asks whether to allow it, then `http`, `https`, `about`, `blob` and `data` URLs are allowed (frames of the player, reCAPTCHA and the login providers need them, and Linux and macOS do not say which frame is navigating), and every other scheme is refused; a `mailto:` URL is refused and opened in the system's default handler.
- **AC-8:** Given the page asks for a new window (`window.open` or a link with `target="_blank"`), when the URL is `App` or `SignIn`, then the popup opens inside the app as a window of its own, related to the main webview so it shares the session and `window.opener`, with the size the page asked for where the engine reports it, else 500 × 650 logical pixels (WebKitGTK does not report it when the popup is created), centered, and titled after its page; when it is `Browser`, the URL opens in the system browser and no window is created; when it is `Refuse`, nothing opens.
- **AC-9:** Given the main frame starts loading a URL classified `Browser`, when the load begins, then the URL opens in the system browser and the main window goes back to the last `App` URL it showed (`https://www.deezer.com/` when there is none).
- **AC-11:** Given a popup opened by AC-8, when its page calls `window.close()` (as a sign-in popup does once it has answered its opener), then its window closes on every platform, and the main window is untouched; the engines disagree on what `window.close()` does to a webview the app created (WebKitGTK destroys the webview and leaves an empty window, WKWebView does nothing), so popups get a script that turns it into a navigation to `https://deezer-tauri.invalid/close`, which the popup's navigation hook answers by closing the window.
- **AC-10:** Given the system browser cannot be opened, when a `Browser` URL is handed to it, then the failure is logged with the URL's scheme and host only, and the app keeps running.

## Out of scope

- Handling `deezer://` links from the operating system.
- An offline page: the engine's own error page is shown when there is no network.
- Auto-update of the app itself (feature 004 publishes releases; updating is the package manager's or the user's).
- Proxy settings, zoom shortcuts, a custom context menu.

## Open questions

- Decided on 2026-10-23: a real Google, Facebook or Apple sign-in is not part of any test. It needs a person's account and, for Google, a password, a code by e-mail and a confirmation on the phone, which exist precisely to stop automation. What the app owns is covered instead: the end-to-end test drives a fixture provider with three steps, each a page of its own in the same popup, which hands its result to the opener and closes (Linux and Windows), and the smoke test presses Google on Deezer's real login page and gets Google's sign-in in an app popup (Linux and Windows). AC-2, AC-3 and AC-4 were verified by hand on GNOME on 2026-10-22.
- Whether Deezer's web player accepts WKWebView with the Safari user agent of AC-5 on every macOS version from 11 up. On the macOS runner (2026-10-23) the smoke test loaded the regular page ("Deezer | 1 Month Free | Music Streaming app online", `https://www.deezer.com/us/`) and the bridge reported through the production capability; signing in and playing on a Mac remain to be seen by someone with one.
