# 002: Media bridge · Tasks

Work in order. Each task is small, testable and linked to the acceptance criteria it covers.

- [x] **T1** · `bridge.js`: idempotent install, handler wrapping, polyfill, hooks, one-second check, artwork choice, transport with retry, `dispatch` · covers AC-1 to AC-6 · test: `src-tauri/scripts/bridge.test.mjs` (`node --test`), one test per AC
- [x] **T2** · `media` module: `MediaSnapshot`, `sanitize`, `PlaybackState`, `MediaAction`, `Player` with `Change`, `Action::script`, `window_title` · covers AC-7, AC-10, AC-11 · test: unit tests in `media.rs`
- [x] **T3** · `bridge` module: `report_media` command, app manifest in `build.rs`, `deezer-remote` capability · covers AC-7, AC-8, AC-9 · test: `tests/ipc.rs` with Tauri's mock runtime (accepted snapshot, malformed payload), `tests/capabilities.rs` reading the capability files
- [x] **T4** · Wire the bridge into the shell: initialization script, title listener, `Player::send` through `eval` · covers AC-11 · test: end-to-end below
- [x] **T5** · `e2e` feature, fixture page and `tests/e2e.rs` · covers AC-12 · test: `make e2e` locally, CI on the three platforms
- [x] **T6** · Update the specs if anything changed during implementation; `make check`
