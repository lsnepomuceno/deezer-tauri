# Deezer Tauri

Deezer Tauri is an unofficial desktop shell for the Deezer web player, in Rust on Tauri 2, for Linux, Windows and macOS. It loads `https://www.deezer.com` in the platform's web engine and adds a tray, track notifications, close-to-tray and a persistent session. It never requests, decrypts or stores audio: see @docs/adr/0002-webview-shell-not-native-client.md before proposing anything that talks to Deezer's APIs.

## Source of truth

This project follows **spec-driven development** (@docs/adr/0001-spec-driven-development.md). Specs are the source of truth; code exists to satisfy them.

- Architecture: @docs/architecture.md
- Feature specs: `specs/features/NNN-name/` with `requirements.md`, `design.md` and `tasks.md` (templates in `specs/features/_template/`).
- Decisions: `docs/adr/`.

## Workflow (mandatory)

1. Never implement behavior that is not described in a spec. If the spec is missing or ambiguous, write or update it first.
2. Do not write code for a feature without a `tasks.md`. Work one task at a time, in order, and tick it when done.
3. Every acceptance criterion (`AC-n`) is covered by at least one test that names it (`001 AC-6`).
4. If implementation reveals a needed change, update the spec in the same change set.
5. A task is done only when `make check` passes: rustfmt, Clippy with warnings as errors, the bridge tests, the unit and IPC tests and the end-to-end test.
6. One branch and one pull request per feature into `main`. CI must be green on Linux, Windows and macOS before merging.

## Architecture rules

- Decisions are pure functions over plain data (`navigation`, `user_agent`, `media`, `settings`, `tray::menu_model`, `notify::decide`, `i18n`); Tauri types appear only in `shell`, `bridge` and the side-effect half of `tray`. A new decision goes into a pure module with unit tests.
- The remote page is untrusted. Its only IPC surface is `report_media`, granted by `capabilities/deezer-remote.json` to `https://*.deezer.com`. Never grant a plugin permission to a remote origin.
- Everything that comes from the page goes through `MediaSnapshot::sanitize` before it reaches a title, a menu or a notification.
- Rust never evaluates text from the page; `MediaAction::script` is the only source of evaluated scripts.
- Media integration with the OS belongs to the engine (@docs/adr/0003-media-session-bridge.md). Do not add MPRIS, SMTC or Now Playing code.
- No DOM selectors of the player. If something is not in the Media Session API, it is out of scope until a spec says how.
- `bridge.js` is a plain script with no build step, tested with `node --test`; keep it ES2019 so every engine runs it.
- Do not log URL paths or queries (login flows carry tokens): scheme and host only.

## Language

- Everything in the repository is in English: identifiers, specs, ADRs, docs, commit messages, comments.
- User-facing strings live in `src-tauri/src/i18n.rs`, in English and Portuguese.
- Commit messages follow Conventional Commits (`feat(tray): …`, `fix(bridge): …`, `docs(specs): …`).

## Commands

Requirements: Rust (stable), Node 20+, and on Linux the Tauri system packages listed in the README.

- `make run`: the app in development
- `make test`: bridge, unit and IPC tests
- `make e2e`: the end-to-end test (under `xvfb-run` on Linux when there is no display)
- `make lint`: rustfmt check and Clippy
- `make check`: everything CI runs, except the installers
- `make bundle`: the installers for the current platform
- `make icons`: regenerate the icons from `assets/`
