# 003: Tray and notifications · Design

## Overview

```
media::Player ──Change──▶ tray::refresh ──▶ tray::menu_model(&PlayerState, &Settings, visible, &Labels) ──▶ Menu
              └─────────▶ notify::decide(&Change, &PlayerState, focused, &Settings) ──▶ Option<Note> ──▶ notification plugin
menu event id ──▶ tray::MenuCommand ──▶ Action (bridge) | toggle window | Settings::set + save | exit
window CloseRequested ──▶ settings.close_to_tray ? prevent_close + hide : exit
```

The tray, the notification plugin and the window are side effects; every decision before them is a pure function over plain data, unit tested.

## Menu model

`tray::menu_model` returns a `Vec<MenuEntry>`, where `MenuEntry` is `Info { id, text }` (the now-playing line), `Item { command, text, enabled }`, `Check { command, text, checked }` or `Separator`. Commands are the closed enum `MenuCommand` (`PlayPause`, `Next`, `Previous`, `ToggleWindow`, `CloseToTray`, `Notifications`, `Quit`) with a stable string form (`play-pause`, …) used as the Tauri menu item id, and parsed back with `MenuCommand::from_id`. The now-playing line has the id `now-playing` and no command.

Native menus (`muda`) read a single `&` as a keyboard mnemonic, so text from the page goes through `tray::literal`, which doubles it (found with an artist named "Hugo & Guilherme").

Tauri menus can be updated item by item, but rebuilding the whole `Menu` and calling `TrayIcon::set_menu` is simpler and cheap at this size (eleven entries), so AC-4 rebuilds.

## Window visibility

The menu's show or hide label comes from the visibility the shell last asked for (`Shell::window_visible`), not from `is_visible()`: under a window manager the engine reports a hide or a show only after the menu has been rebuilt, which left the label inverted when the window was closed to the tray (found on GNOME on 2026-10-22). The shell also keeps the menu it last applied (`applied_menu`), and the end-to-end test reads that one, so it checks what the tray shows rather than recomputing it. Xvfb has no window manager and hides synchronously, so the test guards the logic but cannot reproduce that race; it was verified on GNOME.

## Labels

`i18n::Labels` is a struct of `&'static str` with two constants, `EN` and `PT`, chosen by `i18n::labels_for(locale: Option<&str>)` from `sys-locale`'s value (`pt-BR`, `pt_PT`, `pt` all start with `pt`).

| Key | English | Portuguese |
| --- | --- | --- |
| nothing playing | Nothing playing | Nada tocando |
| play | Play | Tocar |
| pause | Pause | Pausar |
| next | Next | Próxima |
| previous | Previous | Anterior |
| show | Show window | Mostrar janela |
| hide | Hide window | Ocultar janela |
| close to tray | Keep playing when the window is closed | Continuar tocando ao fechar a janela |
| notifications | Track notifications | Notificações de faixa |
| quit | Quit | Sair |

## Settings

`settings::Settings { close_to_tray: bool, notifications: bool }`, serialized camelCase. `Settings::load(path)` never fails (AC-8); `Settings::save(&self, path)` writes `settings.json.tmp` and renames it. The shared copy is `Arc<Mutex<Settings>>` managed by Tauri.

## Notifications

`notify::decide` returns `Some(Note { title, body })` only for AC-7's conditions. The state carries the previous track identity (title, artist, album), so a pause and resume of the same track does not count. The window's focus comes from `WebviewWindow::is_focused`.

## Platform notes

- Linux: the tray is a StatusNotifierItem through `libayatana-appindicator3`; it has no click events, only the menu. GNOME needs the AppIndicator extension (enabled by default on Ubuntu).
- macOS: the icon is set as a template image so it follows the menu bar's light and dark appearance.
- Windows: a primary click toggles the window, the secondary opens the menu (`show_menu_on_left_click(false)`).

## Security

Menu labels and notifications show only sanitized fields (002 AC-7). Notifications carry no URL.

## Performance budget

One menu rebuild per `Change` (at most a few per track); one notification per track at most.

## Alternatives considered

- Updating menu items in place: less churn, more code, and the item handles would have to be kept in sync with the model.
- A settings window: two booleans fit in the tray menu.
