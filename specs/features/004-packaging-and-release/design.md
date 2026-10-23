# 004: Packaging and release · Design

## Overview

Tauri's bundler produces every format from `tauri.conf.json`; the Tauri CLI comes from npm (`@tauri-apps/cli`, pinned in `package.json`) because its prebuilt binaries install in seconds, where `cargo install tauri-cli` compiles for minutes on every CI run.

## Workflows

| Workflow | Trigger | Jobs |
| --- | --- | --- |
| `ci.yml` | pull request to `main`, push to `main`, manual | `checks` matrix (Ubuntu 22.04, Windows, macOS): `make`-equivalent steps, then `tauri build` and artifact upload |
| `release.yml` | tag `v*` | `verify` (tag against versions, then one draft release with the changelog section), `build` matrix (Ubuntu 22.04, Windows, macOS aarch64, macOS x86_64) with `tauri-apps/tauri-action` uploading to that draft by id |

Ubuntu 22.04 is the oldest runner with `libwebkit2gtk-4.1-dev`, and building there keeps the `.deb` and AppImage loadable on older glibc.

The AppImage bundles GStreamer (`bundleMediaFramework`), which needs `patchelf` on the build machine; without the "bad" plugin set (the bundler's default) it played Deezer on 2026-10-22, with the playback position advancing on MPRIS.

The end-to-end test needs a display on Linux: `xvfb-run -a cargo test --features e2e --test e2e`. Windows and macOS runners have a desktop session.

## Windows manifest

`tauri-build` embeds the Windows application manifest (Common Controls v6) in the app binary only; a test binary that links Tauri then fails to start with `STATUS_ENTRYPOINT_NOT_FOUND` (the first CI run on 2026-10-22). `build.rs` disables Tauri's copy and embeds `src-tauri/windows-app-manifest.xml` through the linker (`/MANIFEST:EMBED`, `/MANIFESTINPUT`), which reaches the app, the tests and the examples alike.

## Release notes

`scripts/changelog-section.sh vX.Y.Z` prints the section of `CHANGELOG.md` for that version, without the link reference definitions at the end of the file; the `verify` job uses it as the body of the draft. A tag whose section is missing fails the job.

The draft is created once, in `verify`, and its id goes to every build job. With `tagName` instead, the four jobs of `v0.1.0` each looked for a release that did not exist yet, two created their own, and the installers were split between two drafts (merged by hand before publishing).

## Icons

`assets/icon.svg` (1024 × 1024): a rounded square with a purple gradient (`#a238ff` to `#5b12b8`, the palette listeners associate with Deezer) and a white play symbol with sound arcs; the shape is the project's own, not Deezer's logo. `make icons` renders it to a 1024 px PNG with `@resvg/resvg-js` (`scripts/render-icons.mjs`), runs `tauri icon`, which writes the PNG, ICO and ICNS sizes, and removes the Android and iOS sets `tauri icon` also writes. GNOME matches a window to its launcher by `StartupWMClass=deezer-tauri` in the package's desktop entry, so a binary run straight from `target/` shows a generic icon in the dock while the installed app shows its own; verified on 2026-10-23 on GNOME (Ubuntu 24.04) with the desktop entry, icons and binary of the released `.deb`, which was built on Ubuntu 22.04. A separate monochrome `assets/tray-template.svg` gives the macOS template tray icon and is rendered to `src-tauri/icons/tray-template.png`.

## Security

- Workflows run with `contents: read`, except the release job, which needs `contents: write` to create the draft.
- Third-party actions are pinned to a major version from the vendor (`actions/*`, `tauri-apps/tauri-action`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`).

## Performance budget

- CI wall time per platform: under 20 minutes with the Cargo cache warm.

## Alternatives considered

- `cargo-dist`: built for command-line tools, not for Tauri bundles.
- Building Windows and macOS installers from Linux by cross-compiling: Tauri's MSI and DMG need their native tools.
