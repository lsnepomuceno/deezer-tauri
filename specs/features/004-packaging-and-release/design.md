# 004: Packaging and release · Design

## Overview

Tauri's bundler produces every format from `tauri.conf.json`; the Tauri CLI comes from npm (`@tauri-apps/cli`, pinned in `package.json`) because its prebuilt binaries install in seconds, where `cargo install tauri-cli` compiles for minutes on every CI run.

## Workflows

| Workflow | Trigger | Jobs |
| --- | --- | --- |
| `ci.yml` | pull request to `main`, push to `main`, manual | `checks` matrix (Ubuntu 22.04, Windows, macOS): `make`-equivalent steps, then `tauri build` and artifact upload |
| `release.yml` | tag `v*` | `verify` (tag against versions), `build` matrix (Ubuntu 22.04, Windows, macOS aarch64, macOS x86_64) with `tauri-apps/tauri-action`, draft release with the changelog section |

Ubuntu 22.04 is the oldest runner with `libwebkit2gtk-4.1-dev`, and building there keeps the `.deb` and AppImage loadable on older glibc.

The end-to-end test needs a display on Linux: `xvfb-run -a cargo test --features e2e --test e2e`. Windows and macOS runners have a desktop session.

## Release notes

`scripts/changelog-section.sh vX.Y.Z` prints the section of `CHANGELOG.md` for that version; the release job passes it as the body. A tag whose section is missing fails the job.

## Icons

`assets/icon.svg` (1024 × 1024): a rounded square with a purple gradient (`#a238ff` to `#5b12b8`, the palette listeners associate with Deezer) and a white play symbol with sound arcs; the shape is the project's own, not Deezer's logo. `make icons` renders it to a 1024 px PNG with `@resvg/resvg-js` (`scripts/render-icons.mjs`), runs `tauri icon`, which writes the PNG, ICO and ICNS sizes, and removes the Android and iOS sets `tauri icon` also writes. A separate monochrome `assets/tray-template.svg` gives the macOS template tray icon and is rendered to `src-tauri/icons/tray-template.png`.

## Security

- Workflows run with `contents: read`, except the release job, which needs `contents: write` to create the draft.
- Third-party actions are pinned to a major version from the vendor (`actions/*`, `tauri-apps/tauri-action`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`).

## Performance budget

- CI wall time per platform: under 20 minutes with the Cargo cache warm.

## Alternatives considered

- `cargo-dist`: built for command-line tools, not for Tauri bundles.
- Building Windows and macOS installers from Linux by cross-compiling: Tauri's MSI and DMG need their native tools.
