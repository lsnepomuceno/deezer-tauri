use crate::media::{Change, MediaInfo};
use crate::settings::Settings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub title: String,
    pub body: String,
}

pub fn decide(
    change: Change,
    info: &MediaInfo,
    focused: bool,
    settings: &Settings,
) -> Option<Note> {
    if !settings.notifications || focused || !change.track || !info.is_playing() {
        return None;
    }
    if info.title.is_empty() {
        return None;
    }
    let body = [info.artist.as_str(), info.album.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    Some(Note {
        title: info.title.clone(),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::PlaybackState;

    fn playing(title: &str, artist: &str, album: &str) -> MediaInfo {
        MediaInfo {
            title: title.into(),
            artist: artist.into(),
            album: album.into(),
            playback: PlaybackState::Playing,
            ..Default::default()
        }
    }

    const NEW_TRACK: Change = Change {
        track: true,
        playback: false,
        actions: false,
    };

    #[test]
    fn ac_003_7_a_new_track_in_the_background_notifies() {
        let note = decide(
            NEW_TRACK,
            &playing("Seize the Day", "Avenged Sevenfold", "City of Evil"),
            false,
            &Settings::default(),
        );
        assert_eq!(
            note,
            Some(Note {
                title: "Seize the Day".into(),
                body: "Avenged Sevenfold · City of Evil".into()
            })
        );
    }

    #[test]
    fn ac_003_7_the_body_keeps_whichever_part_exists() {
        let only_artist = decide(
            NEW_TRACK,
            &playing("T", "Artist", ""),
            false,
            &Settings::default(),
        );
        assert_eq!(only_artist.unwrap().body, "Artist");
        let only_album = decide(
            NEW_TRACK,
            &playing("T", "", "Album"),
            false,
            &Settings::default(),
        );
        assert_eq!(only_album.unwrap().body, "Album");
        let neither = decide(
            NEW_TRACK,
            &playing("T", "", ""),
            false,
            &Settings::default(),
        );
        assert_eq!(neither.unwrap().body, "");
    }

    #[test]
    fn ac_003_7_pause_resume_and_actions_never_notify() {
        let info = playing("T", "A", "B");
        for change in [
            Change {
                track: false,
                playback: true,
                actions: false,
            },
            Change {
                track: false,
                playback: false,
                actions: true,
            },
            Change::default(),
        ] {
            assert_eq!(
                decide(change, &info, false, &Settings::default()),
                None,
                "{change:?}"
            );
        }
    }

    #[test]
    fn ac_003_7_a_new_track_while_paused_does_not_notify() {
        let mut info = playing("T", "A", "B");
        info.playback = PlaybackState::Paused;
        assert_eq!(decide(NEW_TRACK, &info, false, &Settings::default()), None);
    }

    #[test]
    fn ac_003_7_focus_or_the_setting_silences_it() {
        let info = playing("T", "A", "B");
        assert_eq!(decide(NEW_TRACK, &info, true, &Settings::default()), None);
        let off = Settings {
            notifications: false,
            ..Settings::default()
        };
        assert_eq!(decide(NEW_TRACK, &info, false, &off), None);
    }

    #[test]
    fn a_track_without_title_does_not_notify() {
        assert_eq!(
            decide(
                NEW_TRACK,
                &playing("", "A", "B"),
                false,
                &Settings::default()
            ),
            None
        );
    }
}
