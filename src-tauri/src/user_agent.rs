pub const MACOS_SAFARI: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
    MacOs,
    Other,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

/// `None` keeps the engine's default.
pub fn for_platform(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::MacOs => Some(MACOS_SAFARI),
        Platform::Linux | Platform::Windows | Platform::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_001_5_linux_and_windows_keep_the_engine_default() {
        assert_eq!(for_platform(Platform::Linux), None);
        assert_eq!(for_platform(Platform::Windows), None);
    }

    #[test]
    fn ac_001_5_macos_presents_itself_as_safari() {
        let agent = for_platform(Platform::MacOs).unwrap();
        assert!(agent.contains("Macintosh"));
        assert!(agent.contains(" Version/"));
        assert!(agent.ends_with(" Safari/605.1.15"));
    }

    #[test]
    fn the_current_platform_is_detected() {
        let expected = if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Other
        };
        assert_eq!(Platform::current(), expected);
    }
}
