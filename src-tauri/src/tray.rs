use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::i18n::Labels;
use crate::media::{window_title, MediaInfo, PageAction, PRODUCT_NAME};
use crate::settings::Settings;

pub const TRAY_ID: &str = "main";
pub const NOW_PLAYING_ID: &str = "now-playing";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    PlayPause,
    Next,
    Previous,
    ToggleWindow,
    CloseToTray,
    Notifications,
    Quit,
}

impl MenuCommand {
    const ALL: [Self; 7] = [
        Self::PlayPause,
        Self::Next,
        Self::Previous,
        Self::ToggleWindow,
        Self::CloseToTray,
        Self::Notifications,
        Self::Quit,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::PlayPause => "play-pause",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::ToggleWindow => "toggle-window",
            Self::CloseToTray => "close-to-tray",
            Self::Notifications => "notifications",
            Self::Quit => "quit",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|command| command.id() == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    Info {
        id: &'static str,
        text: String,
    },
    Item {
        command: MenuCommand,
        text: &'static str,
        enabled: bool,
    },
    Check {
        command: MenuCommand,
        text: &'static str,
        checked: bool,
    },
    Separator,
}

pub fn menu_model(
    info: &MediaInfo,
    settings: &Settings,
    window_visible: bool,
    labels: &Labels,
) -> Vec<MenuEntry> {
    let now_playing = if info.title.is_empty() {
        labels.nothing_playing.to_owned()
    } else {
        window_title(info)
    };
    let (play_pause_text, play_pause_action) = if info.is_playing() {
        (labels.pause, PageAction::Pause)
    } else {
        (labels.play, PageAction::Play)
    };
    vec![
        MenuEntry::Info {
            id: NOW_PLAYING_ID,
            text: now_playing,
        },
        MenuEntry::Item {
            command: MenuCommand::PlayPause,
            text: play_pause_text,
            enabled: info.can(play_pause_action),
        },
        MenuEntry::Item {
            command: MenuCommand::Next,
            text: labels.next,
            enabled: info.can(PageAction::NextTrack),
        },
        MenuEntry::Item {
            command: MenuCommand::Previous,
            text: labels.previous,
            enabled: info.can(PageAction::PreviousTrack),
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            command: MenuCommand::ToggleWindow,
            text: if window_visible {
                labels.hide_window
            } else {
                labels.show_window
            },
            enabled: true,
        },
        MenuEntry::Separator,
        MenuEntry::Check {
            command: MenuCommand::CloseToTray,
            text: labels.close_to_tray,
            checked: settings.close_to_tray,
        },
        MenuEntry::Check {
            command: MenuCommand::Notifications,
            text: labels.notifications,
            checked: settings.notifications,
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            command: MenuCommand::Quit,
            text: labels.quit,
            enabled: true,
        },
    ]
}

/// Native menus read `&` as a mnemonic marker; text from the page must show it as is.
pub fn literal(text: &str) -> String {
    text.replace('&', "&&")
}

pub fn build_menu<R: Runtime>(app: &AppHandle<R>, model: &[MenuEntry]) -> tauri::Result<Menu<R>> {
    let mut items: Vec<Box<dyn IsMenuItem<R>>> = Vec::with_capacity(model.len());
    for entry in model {
        items.push(match entry {
            MenuEntry::Info { id, text } => Box::new(MenuItem::with_id(
                app,
                *id,
                literal(text),
                false,
                None::<&str>,
            )?),
            MenuEntry::Item {
                command,
                text,
                enabled,
            } => Box::new(MenuItem::with_id(
                app,
                command.id(),
                *text,
                *enabled,
                None::<&str>,
            )?),
            MenuEntry::Check {
                command,
                text,
                checked,
            } => Box::new(CheckMenuItem::with_id(
                app,
                command.id(),
                *text,
                true,
                *checked,
                None::<&str>,
            )?),
            MenuEntry::Separator => Box::new(PredefinedMenuItem::separator(app)?),
        });
    }
    let references: Vec<&dyn IsMenuItem<R>> = items.iter().map(AsRef::as_ref).collect();
    Menu::with_items(app, &references)
}

pub fn install<R: Runtime>(
    app: &AppHandle<R>,
    model: &[MenuEntry],
    on_command: impl Fn(&AppHandle<R>, MenuCommand) + Send + Sync + Clone + 'static,
) -> tauri::Result<TrayIcon<R>> {
    let on_click = on_command.clone();
    let builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(PRODUCT_NAME)
        .menu(&build_menu(app, model)?)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            if let Some(command) = MenuCommand::from_id(event.id().as_ref()) {
                on_command(app, command);
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_click(tray.app_handle(), MenuCommand::ToggleWindow);
            }
        });
    let builder = if cfg!(target_os = "macos") {
        builder
            .icon(Image::from_bytes(include_bytes!(
                "../icons/tray-template.png"
            ))?)
            .icon_as_template(true)
    } else {
        match app.default_window_icon() {
            Some(icon) => builder.icon(icon.clone()),
            None => builder,
        }
    };
    builder.build(app)
}

pub fn refresh<R: Runtime>(app: &AppHandle<R>, model: &[MenuEntry]) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(build_menu(app, model)?))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{EN, PT};
    use crate::media::PlaybackState;

    fn info(state: PlaybackState, actions: &[PageAction]) -> MediaInfo {
        MediaInfo {
            title: "Seize the Day".into(),
            artist: "Avenged Sevenfold".into(),
            playback: state,
            actions: actions.iter().copied().collect(),
            ..Default::default()
        }
    }

    fn item(model: &[MenuEntry], wanted: MenuCommand) -> (&'static str, bool) {
        model
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Item {
                    command,
                    text,
                    enabled,
                } if *command == wanted => Some((*text, *enabled)),
                _ => None,
            })
            .unwrap()
    }

    fn check(model: &[MenuEntry], wanted: MenuCommand) -> bool {
        model
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Check {
                    command, checked, ..
                } if *command == wanted => Some(*checked),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn ac_003_1_the_menu_has_every_entry_in_order() {
        let model = menu_model(&MediaInfo::default(), &Settings::default(), true, &EN);
        let shape: Vec<String> = model
            .iter()
            .map(|entry| match entry {
                MenuEntry::Info { id, .. } => (*id).to_owned(),
                MenuEntry::Item { command, .. } | MenuEntry::Check { command, .. } => {
                    command.id().to_owned()
                }
                MenuEntry::Separator => "---".to_owned(),
            })
            .collect();
        assert_eq!(
            shape,
            [
                "now-playing",
                "play-pause",
                "next",
                "previous",
                "---",
                "toggle-window",
                "---",
                "close-to-tray",
                "notifications",
                "---",
                "quit"
            ]
        );
    }

    #[test]
    fn ac_003_2_now_playing_shows_the_track_or_nothing() {
        let playing = menu_model(
            &info(PlaybackState::Playing, &[]),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(
            playing[0],
            MenuEntry::Info {
                id: NOW_PLAYING_ID,
                text: "Seize the Day · Avenged Sevenfold".into()
            }
        );
        let idle = menu_model(&MediaInfo::default(), &Settings::default(), true, &PT);
        assert_eq!(
            idle[0],
            MenuEntry::Info {
                id: NOW_PLAYING_ID,
                text: "Nada tocando".into()
            }
        );
    }

    #[test]
    fn ac_003_2_play_or_pause_follows_the_state_and_the_registered_action() {
        let both = [PageAction::Play, PageAction::Pause];
        let playing = menu_model(
            &info(PlaybackState::Playing, &both),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(item(&playing, MenuCommand::PlayPause), ("Pause", true));

        let paused = menu_model(
            &info(PlaybackState::Paused, &both),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(item(&paused, MenuCommand::PlayPause), ("Play", true));

        let no_pause = menu_model(
            &info(PlaybackState::Playing, &[PageAction::Play]),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(item(&no_pause, MenuCommand::PlayPause), ("Pause", false));

        let idle = menu_model(&MediaInfo::default(), &Settings::default(), true, &EN);
        assert_eq!(item(&idle, MenuCommand::PlayPause), ("Play", false));
    }

    #[test]
    fn ac_003_2_next_and_previous_need_their_actions() {
        let only_next = menu_model(
            &info(PlaybackState::Playing, &[PageAction::NextTrack]),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(item(&only_next, MenuCommand::Next), ("Next", true));
        assert_eq!(item(&only_next, MenuCommand::Previous), ("Previous", false));

        let both = menu_model(
            &info(
                PlaybackState::Playing,
                &[PageAction::NextTrack, PageAction::PreviousTrack],
            ),
            &Settings::default(),
            true,
            &EN,
        );
        assert_eq!(item(&both, MenuCommand::Previous), ("Previous", true));
    }

    #[test]
    fn ac_003_4_window_item_and_checks_follow_visibility_and_settings() {
        let settings = Settings {
            close_to_tray: false,
            notifications: true,
        };
        let visible = menu_model(&MediaInfo::default(), &settings, true, &EN);
        assert_eq!(
            item(&visible, MenuCommand::ToggleWindow),
            ("Hide window", true)
        );
        assert!(!check(&visible, MenuCommand::CloseToTray));
        assert!(check(&visible, MenuCommand::Notifications));

        let hidden = menu_model(&MediaInfo::default(), &settings, false, &PT);
        assert_eq!(
            item(&hidden, MenuCommand::ToggleWindow),
            ("Mostrar janela", true)
        );
    }

    #[test]
    fn ac_003_2_ampersands_from_the_page_are_shown_literally() {
        assert_eq!(literal("Hugo & Guilherme"), "Hugo && Guilherme");
        assert_eq!(literal("&&"), "&&&&");
        assert_eq!(literal("No mnemonic"), "No mnemonic");
    }

    #[test]
    fn ac_003_3_every_command_id_round_trips() {
        for command in MenuCommand::ALL {
            assert_eq!(MenuCommand::from_id(command.id()), Some(command));
        }
        assert_eq!(MenuCommand::from_id(NOW_PLAYING_ID), None);
        assert_eq!(MenuCommand::from_id("unknown"), None);
    }
}
