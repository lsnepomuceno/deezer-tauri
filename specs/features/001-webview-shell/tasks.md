# 001: Webview shell · Tasks

Work in order. Each task is small, testable and linked to the acceptance criteria it covers.

- [x] **T1** · Project skeleton: `src-tauri` crate, `tauri.conf.json` without declared windows, empty capability, placeholder icons, Makefile targets `run`, `test`, `lint`, `check` · covers none · test: `make check`
- [x] **T2** · `navigation` module: `Policy`, `classify`, `allow_frame_navigation`, `new_window`, `main_frame_load` · covers AC-6, AC-7, AC-8, AC-9 · test: unit tests in `navigation.rs` named after each AC, including the look-alike hosts
- [x] **T3** · `user_agent` module · covers AC-5 · test: unit tests per platform
- [x] **T4** · `shell` wiring: main window with the three hooks, `opener` for external URLs with scheme-and-host logging, last `App` URL kept for AC-9, `single-instance` and `window-state` plugins · covers AC-1 to AC-4, AC-8 to AC-10 · test: end-to-end run of feature 002 (window created, fixture loaded); manual check on Linux of AC-2, AC-3, AC-4 and of a Google sign-in popup
- [x] **T5** · AC-7, AC-8 and AC-9 in the real engines: `mailto:`, a page outside Deezer and two popups driven by the end-to-end harness, with external URLs recorded instead of opened · covers AC-7, AC-8, AC-9 · test: `tests/e2e.rs` on Linux, Windows and macOS (popups on Linux and Windows), each scenario proven to fail when its rule is broken
- [x] **T6** · Update the specs if anything changed during implementation; `make check`
