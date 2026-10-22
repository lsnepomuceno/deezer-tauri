# 003: Tray and notifications

## Problem

A music player lives in the background. Closing the window of a browser tab stops the music; minimizing leaves a window in the way. The listener wants the player out of sight but in reach: an icon in the tray with the controls, the window back with one click, a note when the track changes, and the music not stopping because the window was closed.

## User stories

- As a listener, I want to close the window and keep listening, and to quit only when I choose to.
- As a listener, I want play, pause, next and previous in the tray menu, with the track that is playing, so that I control the music without opening the window.
- As a listener, I want a desktop notification when a new track starts while the window is in the background, and to turn it off.
- As a Portuguese speaker, I want the menu in Portuguese; others want it in English.

## Acceptance criteria

- **AC-1:** Given the app starts, when the tray is created, then it shows the app icon with the tooltip `Deezer Tauri` and a menu with, in order: the now-playing line (disabled), play or pause, next, previous, a separator, show or hide window, a separator, close to tray (check), track notifications (check), a separator, quit.
- **AC-2:** Given the player state, when the menu model is built, then the now-playing line is `<title> · <artist>` (or the title alone) and `Nothing playing` when there is no title, with any `&` in it shown as a literal ampersand (native menus read a single `&` as a keyboard mnemonic); the play-or-pause item reads `Pause` while `playing` and `Play` otherwise, and is enabled only when the matching action (`pause` or `play`) is registered; next and previous are enabled only when `nexttrack` and `previoustrack` are registered.
- **AC-3:** Given a menu item is chosen, when it is play or pause, next or previous, then the matching action (`Toggle`, `Next`, `Previous`) is sent through the bridge (002 AC-10); show or hide toggles the window (showing also unminimizes and focuses it); the checks flip and save their setting; quit exits the app.
- **AC-4:** Given the state or the window visibility changes, when it does, then the menu is rebuilt from the model (labels, enabled flags and check states), without recreating the tray icon.
- **AC-5:** Given close to tray is on (the default), when the user closes the main window, then it hides and playback continues; when it is off, closing the window quits the app.
- **AC-6:** Given the tray icon is clicked with the primary button, when the platform reports clicks (Windows and macOS), then the window is toggled as in AC-3; on Linux, where the StatusNotifier tray only opens the menu, the menu is the way to do it.
- **AC-7:** Given track notifications are on (the default), when the player reports a new track (title, artist or album differ from the previous track) while `playing` and the main window is not focused, then a desktop notification shows the title as its heading and `<artist> · <album>` (or whichever of the two exists) as its body; pausing, resuming or a change of actions alone never notifies; when notifications are off or the window is focused, nothing is shown.
- **AC-8:** Given the settings file, when the app starts, then it reads `closeToTray` and `notifications` from `settings.json` in the app config directory; a missing file, invalid JSON or a missing key yields the default (`true`) for that key, logs a warning for invalid JSON, and never fails startup; a change is written atomically (temporary file and rename).
- **AC-9:** Given the system locale, when labels are chosen, then a locale whose language is `pt` uses Portuguese and anything else uses English, for every label of AC-1 and AC-2.

## Out of scope

- Global keyboard shortcuts (the engine's own media integration handles media keys).
- Starting minimized or at login.
- Notification actions (buttons inside the notification).
- Artwork inside the notification: it would mean downloading images from the page's URLs into the user's machine.

## Open questions

- None.
