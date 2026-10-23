# 004: Packaging and release

## Problem

The app has to install like any other on Linux, Windows and macOS, and every change has to be proven on all three before it reaches a release, since the maintainer runs only Linux day to day.

## User stories

- As a Linux user, I want a `.deb`, an `.rpm` and an AppImage, so that I install it the way my distribution expects.
- As a Windows user, I want an installer that also brings WebView2 when it is missing.
- As a macOS user, I want a `.dmg` for Apple Silicon and one for Intel.
- As a maintainer, I want CI to fail a pull request that breaks any platform, and a tag to produce the release.

## Acceptance criteria

- **AC-1:** Given the bundle configuration, when the app is built for release, then it produces `.deb`, `.rpm` and AppImage on Linux, NSIS `.exe` and `.msi` on Windows, and `.dmg` and `.app` on macOS, with the product name `Deezer Tauri`, the identifier `io.github.lsnepomuceno.deezertauri`, the version of `src-tauri/Cargo.toml`, the category `Music`, and the project's own icon in every size the platforms need.
- **AC-2:** Given the Linux packages, when they are installed, then they depend on `libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1` and the GStreamer plugins the player needs (`gstreamer1.0-plugins-good`, `gstreamer1.0-plugins-bad`, `gstreamer1.0-libav` for `.deb`; their Fedora names for `.rpm`).
- **AC-3:** Given the Windows installer, when WebView2 is missing, then the installer downloads and installs it (bootstrapper mode).
- **AC-4:** Given the macOS bundle, when it is built, then its minimum system version is 11.0.
- **AC-5:** Given a pull request or a push to `main`, when CI runs, then on each of Ubuntu 22.04, Windows and macOS it checks formatting, runs Clippy with warnings as errors, runs the bridge tests, the unit and IPC tests and the end-to-end test (under Xvfb on Linux), and builds the installers, uploading them as workflow artifacts; any failure fails the run.
- **AC-6:** Given a tag `vX.Y.Z`, when it is pushed, then a workflow checks that the tag matches the version in `Cargo.toml` and `tauri.conf.json`, builds the installers on the three platforms (macOS for `aarch64` and `x86_64`) and attaches them to a draft GitHub release named after the tag, with the notes of that version from `CHANGELOG.md`.
- **AC-7:** Given the icon source `assets/icon.svg`, when `make icons` runs, then every icon in `src-tauri/icons` is regenerated from it; the icon is the project's own and does not reproduce Deezer's logo.
- **AC-9:** Given the smoke workflow, when it runs (by hand, every Monday, and on a pull request that touches `src-tauri`), then on Ubuntu 22.04, Windows and macOS it loads `https://www.deezer.com/` in the real engine and passes only when the page loads, the bridge reports through the production capability and the window stays on Deezer; it is separate from CI because it depends on the network and on Deezer.
- **AC-8:** Given dependency updates, when Dependabot runs monthly, then it opens grouped pull requests for Cargo, npm and GitHub Actions.

## Out of scope

- Code signing and notarization: the binaries are unsigned. The README explains the SmartScreen prompt on Windows and the Gatekeeper prompt on macOS.
- Flatpak, Snap, Homebrew, winget and AUR packaging.
- In-app updates.

## Open questions

- None.
