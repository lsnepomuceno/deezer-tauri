use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use url::Url;

pub const PRODUCT_NAME: &str = "Deezer Tauri";

const MAX_TEXT_CHARS: usize = 300;
const MAX_ARTWORK_BYTES: usize = 2048;

/// What `bridge.js` sends, before any validation.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaSnapshot {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: Option<String>,
    pub playback_state: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    None,
    Paused,
    Playing,
}

impl PlaybackState {
    fn parse(raw: &str) -> Self {
        match raw {
            "playing" => Self::Playing,
            "paused" => Self::Paused,
            _ => Self::None,
        }
    }
}

/// The actions of the Media Session API a page can register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageAction {
    Play,
    Pause,
    Stop,
    SeekBackward,
    SeekForward,
    SeekTo,
    PreviousTrack,
    NextTrack,
    SkipAd,
    ToggleMicrophone,
    ToggleCamera,
    HangUp,
    PreviousSlide,
    NextSlide,
    EnterPictureInPicture,
    VoiceActivity,
}

impl PageAction {
    fn parse(raw: &str) -> Option<Self> {
        Some(match raw {
            "play" => Self::Play,
            "pause" => Self::Pause,
            "stop" => Self::Stop,
            "seekbackward" => Self::SeekBackward,
            "seekforward" => Self::SeekForward,
            "seekto" => Self::SeekTo,
            "previoustrack" => Self::PreviousTrack,
            "nexttrack" => Self::NextTrack,
            "skipad" => Self::SkipAd,
            "togglemicrophone" => Self::ToggleMicrophone,
            "togglecamera" => Self::ToggleCamera,
            "hangup" => Self::HangUp,
            "previousslide" => Self::PreviousSlide,
            "nextslide" => Self::NextSlide,
            "enterpictureinpicture" => Self::EnterPictureInPicture,
            "voiceactivity" => Self::VoiceActivity,
            _ => return None,
        })
    }
}

/// A snapshot after validation: the only form the rest of the app sees.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub artwork: Option<String>,
    pub playback: PlaybackState,
    pub actions: BTreeSet<PageAction>,
}

impl MediaInfo {
    pub fn can(&self, action: PageAction) -> bool {
        self.actions.contains(&action)
    }

    pub fn is_playing(&self) -> bool {
        self.playback == PlaybackState::Playing
    }

    fn same_track(&self, other: &Self) -> bool {
        self.title == other.title && self.artist == other.artist && self.album == other.album
    }
}

impl MediaSnapshot {
    pub fn sanitize(self) -> MediaInfo {
        MediaInfo {
            title: clean_text(&self.title),
            artist: clean_text(&self.artist),
            album: clean_text(&self.album),
            artwork: self.artwork.as_deref().and_then(clean_artwork),
            playback: PlaybackState::parse(&self.playback_state),
            actions: self
                .actions
                .iter()
                .filter_map(|raw| PageAction::parse(raw))
                .collect(),
        }
    }
}

fn clean_text(raw: &str) -> String {
    let spaced: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_TEXT_CHARS)
        .collect()
}

fn clean_artwork(raw: &str) -> Option<String> {
    if raw.len() > MAX_ARTWORK_BYTES {
        return None;
    }
    let url = Url::parse(raw).ok()?;
    (url.scheme() == "https").then(|| url.to_string())
}

/// Something the shell asks the player to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAction {
    Toggle,
    Play,
    Pause,
    Next,
    Previous,
}

impl MediaAction {
    pub fn name(self) -> &'static str {
        match self {
            Self::Toggle => "toggle",
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Next => "nexttrack",
            Self::Previous => "previoustrack",
        }
    }

    /// The only scripts the shell ever evaluates in the page.
    pub fn script(self) -> String {
        format!(
            "window.__deezerTauri && window.__deezerTauri.dispatch(\"{}\");",
            self.name()
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    pub track: bool,
    pub playback: bool,
    pub actions: bool,
}

impl Change {
    pub fn between(previous: &MediaInfo, next: &MediaInfo) -> Self {
        Self {
            track: !previous.same_track(next),
            playback: previous.playback != next.playback,
            actions: previous.actions != next.actions,
        }
    }

    pub fn is_empty(self) -> bool {
        !(self.track || self.playback || self.actions)
    }
}

pub fn window_title(info: &MediaInfo) -> String {
    match (info.title.is_empty(), info.artist.is_empty()) {
        (true, _) => PRODUCT_NAME.to_owned(),
        (false, true) => info.title.clone(),
        (false, false) => format!("{} · {}", info.title, info.artist),
    }
}

type Listener = Box<dyn Fn(&MediaInfo, Change) + Send + Sync>;

/// The player's state as the shell knows it, and who to tell when it changes.
#[derive(Default)]
pub struct Player {
    state: Mutex<MediaInfo>,
    listeners: Mutex<Vec<Arc<Listener>>>,
    reports: AtomicUsize,
}

impl Player {
    pub fn current(&self) -> MediaInfo {
        lock(&self.state).clone()
    }

    pub fn subscribe(&self, listener: impl Fn(&MediaInfo, Change) + Send + Sync + 'static) {
        lock(&self.listeners).push(Arc::new(Box::new(listener)));
    }

    /// How many snapshots the page has sent, changed or not.
    pub fn reports(&self) -> usize {
        self.reports.load(Ordering::SeqCst)
    }

    pub fn update(&self, next: MediaInfo) -> Change {
        self.reports.fetch_add(1, Ordering::SeqCst);
        let change = {
            let mut state = lock(&self.state);
            let change = Change::between(&state, &next);
            *state = next.clone();
            change
        };
        if !change.is_empty() {
            let listeners: Vec<_> = lock(&self.listeners).clone();
            for listener in listeners {
                listener(&next, change);
            }
        }
        change
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(title: &str, state: &str, actions: &[&str]) -> MediaSnapshot {
        MediaSnapshot {
            title: title.into(),
            artist: "Artist".into(),
            album: "Album".into(),
            artwork: None,
            playback_state: state.into(),
            actions: actions.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    #[test]
    fn ac_002_7_text_is_trimmed_stripped_of_controls_and_cut_at_300_chars() {
        let info = MediaSnapshot {
            title: "  Seize\nthe\tDay\u{0007}  ".into(),
            artist: "é".repeat(400),
            album: "\u{001b}[31mRed".into(),
            ..Default::default()
        }
        .sanitize();
        assert_eq!(info.title, "Seize the Day");
        assert_eq!(info.artist.chars().count(), 300);
        assert!(info.artist.chars().all(|c| c == 'é'));
        assert_eq!(info.album, "[31mRed");
    }

    #[test]
    fn ac_002_7_artwork_must_be_https_and_short() {
        let with = |artwork: &str| {
            MediaSnapshot {
                artwork: Some(artwork.into()),
                ..Default::default()
            }
            .sanitize()
            .artwork
        };
        assert_eq!(
            with("https://cdn-images.dzcdn.net/images/cover/x/500x500.jpg").as_deref(),
            Some("https://cdn-images.dzcdn.net/images/cover/x/500x500.jpg")
        );
        assert_eq!(with("http://cdn-images.dzcdn.net/x.jpg"), None);
        assert_eq!(with("file:///etc/passwd"), None);
        assert_eq!(with("javascript:alert(1)"), None);
        assert_eq!(with("not a url"), None);
        assert_eq!(with(&format!("https://a.b/{}", "x".repeat(2100))), None);
    }

    #[test]
    fn ac_002_7_unknown_playback_states_read_as_none() {
        assert_eq!(
            snapshot("t", "playing", &[]).sanitize().playback,
            PlaybackState::Playing
        );
        assert_eq!(
            snapshot("t", "paused", &[]).sanitize().playback,
            PlaybackState::Paused
        );
        assert_eq!(
            snapshot("t", "none", &[]).sanitize().playback,
            PlaybackState::None
        );
        assert_eq!(
            snapshot("t", "PLAYING", &[]).sanitize().playback,
            PlaybackState::None
        );
        assert_eq!(
            snapshot("t", "", &[]).sanitize().playback,
            PlaybackState::None
        );
    }

    #[test]
    fn ac_002_7_unknown_actions_are_dropped_and_at_most_16_kept() {
        let mut actions = vec!["play", "nexttrack", "hack", "PLAY", "eval"];
        let all = [
            "play",
            "pause",
            "stop",
            "seekbackward",
            "seekforward",
            "seekto",
            "previoustrack",
            "nexttrack",
            "skipad",
            "togglemicrophone",
            "togglecamera",
            "hangup",
            "previousslide",
            "nextslide",
            "enterpictureinpicture",
            "voiceactivity",
        ];
        actions.extend(all);
        actions.extend(all);
        let info = snapshot("t", "playing", &actions).sanitize();
        assert_eq!(info.actions.len(), 16);
        assert!(info.can(PageAction::Play));
        assert!(info.can(PageAction::VoiceActivity));
    }

    #[test]
    fn ac_002_8_a_payload_of_the_wrong_shape_does_not_deserialize() {
        assert!(serde_json::from_str::<MediaSnapshot>(r#"{"title": 42}"#).is_err());
        assert!(serde_json::from_str::<MediaSnapshot>(r#"{"actions": "play"}"#).is_err());
        assert!(serde_json::from_str::<MediaSnapshot>(r#""title""#).is_err());
        let partial: MediaSnapshot = serde_json::from_str(r#"{"title": "Only"}"#).unwrap();
        assert_eq!(partial.sanitize().title, "Only");
    }

    #[test]
    fn ac_002_10_each_action_evaluates_a_fixed_dispatch_call() {
        let expected = [
            (MediaAction::Toggle, "toggle"),
            (MediaAction::Play, "play"),
            (MediaAction::Pause, "pause"),
            (MediaAction::Next, "nexttrack"),
            (MediaAction::Previous, "previoustrack"),
        ];
        for (action, name) in expected {
            assert_eq!(
                action.script(),
                format!("window.__deezerTauri && window.__deezerTauri.dispatch(\"{name}\");")
            );
        }
    }

    #[test]
    fn ac_002_11_window_title_follows_the_track() {
        let mut info = MediaInfo::default();
        assert_eq!(window_title(&info), "Deezer Tauri");
        info.title = "Seize the Day".into();
        assert_eq!(window_title(&info), "Seize the Day");
        info.artist = "Avenged Sevenfold".into();
        assert_eq!(window_title(&info), "Seize the Day · Avenged Sevenfold");
    }

    #[test]
    fn ac_002_7_listeners_hear_only_real_changes() {
        let player = Player::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&calls);
        player.subscribe(move |_, _| {
            counted.fetch_add(1, Ordering::SeqCst);
        });

        let first = player.update(snapshot("One", "playing", &["play", "pause"]).sanitize());
        assert_eq!(
            first,
            Change {
                track: true,
                playback: true,
                actions: true
            }
        );

        let same = player.update(snapshot("One", "playing", &["pause", "play"]).sanitize());
        assert!(same.is_empty());

        let paused = player.update(snapshot("One", "paused", &["play", "pause"]).sanitize());
        assert_eq!(
            paused,
            Change {
                track: false,
                playback: true,
                actions: false
            }
        );

        let next = player.update(snapshot("Two", "paused", &["play", "pause"]).sanitize());
        assert_eq!(
            next,
            Change {
                track: true,
                playback: false,
                actions: false
            }
        );

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(player.reports(), 4);
        assert_eq!(player.current().title, "Two");
    }

    #[test]
    fn a_listener_may_read_the_player_without_deadlocking() {
        let player = Arc::new(Player::default());
        let inner = Arc::clone(&player);
        let seen = Arc::new(Mutex::new(String::new()));
        let writer = Arc::clone(&seen);
        player.subscribe(move |_, _| {
            *writer.lock().unwrap() = inner.current().title;
        });
        player.update(snapshot("Reentrant", "playing", &[]).sanitize());
        assert_eq!(*seen.lock().unwrap(), "Reentrant");
    }
}
