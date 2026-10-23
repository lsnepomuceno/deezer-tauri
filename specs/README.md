# Specs

Specs are the source of truth for Deezer Tauri. Code implements them; tests prove them on every platform.

| Folder | Content |
| --- | --- |
| `features/NNN-name/` | One folder per feature: `requirements.md`, `design.md`, `tasks.md` |
| `features/_template/` | The three files to copy when a feature starts |

The architecture lives in [`../docs/architecture.md`](../docs/architecture.md) and the decisions in [`../docs/adr/`](../docs/adr/).

## Features

| Feature | Status |
| --- | --- |
| [`001-webview-shell`](features/001-webview-shell/requirements.md) | done |
| [`002-media-bridge`](features/002-media-bridge/requirements.md) | done |
| [`003-tray-and-notifications`](features/003-tray-and-notifications/requirements.md) | done |
| [`004-packaging-and-release`](features/004-packaging-and-release/requirements.md) | done |

## Conventions

- Acceptance criteria are numbered per feature (`AC-1`, `AC-2`, …) and never renumbered; a removed criterion keeps its number with the text "Removed: reason".
- A test names the criterion it covers in its name or in a comment on the line above it (`001 AC-6`), so `grep -rn "001 AC-6"` finds it.
- A change in behavior updates the spec in the same commit.
