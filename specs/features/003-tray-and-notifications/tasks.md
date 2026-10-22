# 003: Tray and notifications · Tasks

Work in order. Each task is small, testable and linked to the acceptance criteria it covers.

- [x] **T1** · `settings` module: defaults, tolerant load, atomic save · covers AC-8 · test: unit tests with `tempfile`
- [x] **T2** · `i18n` module: `Labels`, `labels_for` · covers AC-9 · test: unit tests per locale form
- [x] **T3** · `tray` model: `MenuCommand`, `MenuEntry`, `menu_model` · covers AC-1, AC-2 · test: unit tests for every combination of state, actions and visibility
- [x] **T4** · `notify::decide` · covers AC-7 · test: unit tests (new track, pause and resume, focus, setting off, actions only)
- [x] **T5** · Wire the tray, menu events, close-to-tray and notifications into the shell · covers AC-3 to AC-6 · test: end-to-end run builds the tray and drives the actions through the same `MenuCommand` path; manual check on Linux of the menu, close-to-tray and a notification
- [x] **T6** · Update the specs if anything changed during implementation; `make check`
