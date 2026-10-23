# 004: Packaging and release · Tasks

Work in order. Each task is small, testable and linked to the acceptance criteria it covers.

- [x] **T1** · Icon sources and `make icons` · covers AC-7 · test: `make icons` regenerates `src-tauri/icons` with no diff on a second run
- [x] **T2** · Bundle configuration (targets, identifier, category, Linux dependencies, WebView2 mode, macOS minimum) · covers AC-1 to AC-4 · test: `make bundle` on Linux produces `.deb`, `.rpm` and AppImage; `dpkg-deb -I` lists the dependencies; the `.deb` installs and starts
- [x] **T3** · `ci.yml` · covers AC-5 · test: the first run after publishing is green on the three platforms
- [x] **T4** · `scripts/changelog-section.sh` and `release.yml` · covers AC-6 · test: the script against `CHANGELOG.md` locally; the first tag produces a draft release with the installers
- [x] **T5** · `dependabot.yml` · covers AC-8 · test: GitHub accepts the file (Insights, Dependency graph, Dependabot)
- [x] **T6** · Update the specs if anything changed during implementation; `make check`
