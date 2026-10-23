# Contributing

Thanks for helping. Issues and pull requests are welcome, in English or Portuguese.

## How the project works

Deezer Tauri follows spec-driven development ([ADR 0001](docs/adr/0001-spec-driven-development.md)):

1. A change in behavior starts in `specs/features/NNN-name/` (`requirements.md`, `design.md`, `tasks.md`). A new feature copies `specs/features/_template/`.
2. Every acceptance criterion has an ID (`AC-3`) and at least one test that names it (`002 AC-3`).
3. The spec, the code and the tests change in the same pull request.

Read [`docs/architecture.md`](docs/architecture.md) first, and [ADR 0002](docs/adr/0002-webview-shell-not-native-client.md) before proposing anything that talks to Deezer's APIs: the project never requests, decrypts or stores audio, and such a change will not be accepted.

## Setting up

You need Rust (stable), Node 20 or newer and, on Linux, the packages in the README's "Building from source".

```sh
make deps      # Node tools: the Tauri CLI and the icon renderer
make run       # the app in development
make check     # everything CI runs: formatting, Clippy, bridge, unit, IPC and end-to-end tests
make bundle    # the installers for your platform
```

On Linux, `make e2e` uses `xvfb-run` when it is installed, so the test window stays off your desktop, and `xdotool` to click the page for the popup steps. `make smoke` loads Deezer itself and needs the network.

## Pull requests

- One feature or fix per pull request, into `main`, with `make check` green locally.
- CI runs on Linux, Windows and macOS; all three must pass.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) in English: `feat(tray): …`, `fix(bridge): …`, `docs(specs): …`.
- User-facing text goes in `src-tauri/src/i18n.rs`, in English and Portuguese.
- Add a line to `CHANGELOG.md` under `[Unreleased]` when users will notice the change.

## Releasing

1. Move the `[Unreleased]` notes of `CHANGELOG.md` under the new version and date.
2. Set the same version in `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
3. Tag `vX.Y.Z` on `main` and push the tag. The release workflow checks the versions, builds every installer and opens a draft release with the changelog section; review it and publish.
