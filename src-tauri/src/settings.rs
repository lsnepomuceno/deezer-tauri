use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub close_to_tray: bool,
    pub notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            notifications: true,
        }
    }
}

impl Settings {
    /// Never fails: whatever cannot be read falls back to the default.
    pub fn load(path: &Path) -> Self {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                log::warn!("settings unreadable, using defaults: {error}");
                return Self::default();
            }
        };
        serde_json::from_str(&raw).unwrap_or_else(|error| {
            log::warn!("settings invalid, using defaults: {error}");
            Self::default()
        })
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, json)?;
        fs::rename(&temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn ac_003_8_a_missing_file_gives_the_defaults() {
        let dir = dir();
        let settings = Settings::load(&dir.path().join(FILE_NAME));
        assert_eq!(
            settings,
            Settings {
                close_to_tray: true,
                notifications: true
            }
        );
    }

    #[test]
    fn ac_003_8_invalid_json_gives_the_defaults() {
        let dir = dir();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        fs::write(&path, r#"{"closeToTray": "yes"}"#).unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn ac_003_8_a_missing_key_defaults_alone() {
        let dir = dir();
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, r#"{"notifications": false}"#).unwrap();
        assert_eq!(
            Settings::load(&path),
            Settings {
                close_to_tray: true,
                notifications: false
            }
        );
    }

    #[test]
    fn ac_003_8_saving_round_trips_and_leaves_no_temporary_file() {
        let dir = dir();
        let path = dir.path().join("nested").join(FILE_NAME);
        let settings = Settings {
            close_to_tray: false,
            notifications: false,
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        assert!(!path.with_extension("json.tmp").exists());
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"closeToTray\": false"));
    }
}
