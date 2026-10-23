use url::Url;

pub const START_URL: &str = "https://www.deezer.com/";

/// Where a popup's `window.close()` is sent, so the shell can close its window (001 AC-11).
/// `.invalid` never resolves, so the navigation can only end in the shell's hands.
pub const CLOSE_REQUEST_URL: &str = "https://deezer-tauri.invalid/close";

/// Injected into popups: the engines disagree on `window.close()` for a webview the app
/// created, so it becomes a navigation the popup's hook understands.
pub const POPUP_SCRIPT: &str = r#"(function () {
  if (!window.opener) { return; }
  var request = function () { window.location.href = "https://deezer-tauri.invalid/close"; };
  try { window.close = request; } catch (error) {}
})();"#;

const APP_DOMAINS: &[&str] = &["deezer.com"];

const SIGN_IN_HOSTS: &[&str] = &[
    "accounts.google.com",
    "accounts.youtube.com",
    "www.facebook.com",
    "m.facebook.com",
    "facebook.com",
    "appleid.apple.com",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    App,
    SignIn,
    Browser,
    Refuse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecision {
    Allow,
    Refuse,
    OpenExternally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupDecision {
    OpenInApp,
    OpenExternally,
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadDecision {
    Stay,
    LeaveToBrowser,
}

#[derive(Debug, Clone, Default)]
pub struct Policy {
    extra_app_origins: Vec<url::Origin>,
}

impl Policy {
    pub fn with_app_origin(mut self, url: &Url) -> Self {
        self.extra_app_origins.push(url.origin());
        self
    }

    pub fn classify(&self, url: &Url) -> Destination {
        match url.scheme() {
            "http" | "https" => self.classify_web(url),
            "about" if matches!(url.path(), "blank" | "srcdoc") => Destination::App,
            "mailto" => Destination::Browser,
            _ => Destination::Refuse,
        }
    }

    pub fn classify_str(&self, raw: &str) -> Destination {
        Url::parse(raw).map_or(Destination::Refuse, |url| self.classify(&url))
    }

    pub fn frame_navigation(&self, url: &Url) -> FrameDecision {
        match url.scheme() {
            "http" | "https" | "about" | "blob" | "data" => FrameDecision::Allow,
            "mailto" => FrameDecision::OpenExternally,
            _ => FrameDecision::Refuse,
        }
    }

    pub fn new_window(&self, url: &Url) -> PopupDecision {
        match self.classify(url) {
            Destination::App | Destination::SignIn => PopupDecision::OpenInApp,
            Destination::Browser => PopupDecision::OpenExternally,
            Destination::Refuse => PopupDecision::Ignore,
        }
    }

    pub fn main_frame_load(&self, url: &Url) -> LoadDecision {
        match self.classify(url) {
            Destination::Browser => LoadDecision::LeaveToBrowser,
            _ => LoadDecision::Stay,
        }
    }

    fn classify_web(&self, url: &Url) -> Destination {
        if self.extra_app_origins.contains(&url.origin()) {
            return Destination::App;
        }
        let Some(host) = url.host_str() else {
            return Destination::Refuse;
        };
        if APP_DOMAINS.iter().any(|domain| on_domain(host, domain)) {
            return Destination::App;
        }
        if url.scheme() == "https" && SIGN_IN_HOSTS.contains(&host) {
            return Destination::SignIn;
        }
        Destination::Browser
    }
}

fn on_domain(host: &str, domain: &str) -> bool {
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

pub fn is_close_request(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("deezer-tauri.invalid")
        && url.path() == "/close"
}

/// Scheme and host only: paths and queries of login flows can carry tokens.
pub fn describe(url: &Url) -> String {
    match url.host_str() {
        Some(host) => format!("{}://{}", url.scheme(), host),
        None => format!("{}:", url.scheme()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(raw: &str) -> Destination {
        Policy::default().classify_str(raw)
    }

    fn url(raw: &str) -> Url {
        Url::parse(raw).unwrap()
    }

    #[test]
    fn ac_001_6_deezer_and_its_subdomains_are_the_app() {
        for raw in [
            "https://www.deezer.com/br/",
            "https://deezer.com",
            "http://www.deezer.com/en/album/1",
            "https://account.deezer.com/login",
            "https://WWW.DEEZER.COM/",
            "https://www.deezer.com:8443/",
            "about:blank",
            "about:srcdoc",
        ] {
            assert_eq!(classify(raw), Destination::App, "{raw}");
        }
    }

    #[test]
    fn ac_001_6_login_providers_are_sign_in_over_https_only() {
        for raw in [
            "https://accounts.google.com/o/oauth2/auth?client_id=x",
            "https://accounts.youtube.com/accounts/SetSID",
            "https://www.facebook.com/dialog/oauth",
            "https://m.facebook.com/login",
            "https://facebook.com/login",
            "https://appleid.apple.com/auth/authorize",
        ] {
            assert_eq!(classify(raw), Destination::SignIn, "{raw}");
        }
        assert_eq!(
            classify("http://accounts.google.com/"),
            Destination::Browser
        );
    }

    #[test]
    fn ac_001_6_look_alike_hosts_are_not_deezer() {
        for raw in [
            "https://evil-deezer.com/",
            "https://deezer.com.evil.net/",
            "https://notdeezer.com/",
            "https://deezer.co/",
            "https://evil.com/?next=deezer.com",
            "https://www.facebook.com.evil.net/",
        ] {
            assert_eq!(classify(raw), Destination::Browser, "{raw}");
        }
    }

    #[test]
    fn ac_001_6_other_web_urls_and_mailto_go_to_the_browser() {
        assert_eq!(classify("https://example.org/artist"), Destination::Browser);
        assert_eq!(classify("mailto:support@deezer.com"), Destination::Browser);
    }

    #[test]
    fn ac_001_6_other_schemes_and_garbage_are_refused() {
        for raw in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/html,hi",
            "deezer://track/1",
            "about:config",
            "ftp://example.org/",
            "not a url",
            "",
        ] {
            assert_eq!(classify(raw), Destination::Refuse, "{raw:?}");
        }
    }

    #[test]
    fn ac_001_7_frames_may_load_any_web_scheme() {
        let policy = Policy::default();
        for raw in [
            "https://www.google.com/recaptcha/api2/anchor",
            "https://www.deezer.com/",
            "http://example.org/",
            "about:blank",
            "blob:https://www.deezer.com/5c1e",
            "data:text/html,frame",
        ] {
            assert_eq!(
                policy.frame_navigation(&url(raw)),
                FrameDecision::Allow,
                "{raw}"
            );
        }
    }

    #[test]
    fn ac_001_7_other_schemes_are_refused_and_mailto_goes_to_the_system() {
        let policy = Policy::default();
        assert_eq!(
            policy.frame_navigation(&url("mailto:a@b.c")),
            FrameDecision::OpenExternally
        );
        for raw in ["file:///tmp/x", "deezer://open", "intent://x"] {
            assert_eq!(
                policy.frame_navigation(&url(raw)),
                FrameDecision::Refuse,
                "{raw}"
            );
        }
    }

    #[test]
    fn ac_001_8_popups_open_in_app_for_deezer_and_sign_in() {
        let policy = Policy::default();
        assert_eq!(
            policy.new_window(&url("https://accounts.google.com/o/oauth2/auth")),
            PopupDecision::OpenInApp
        );
        assert_eq!(
            policy.new_window(&url("https://www.deezer.com/br/payment")),
            PopupDecision::OpenInApp
        );
    }

    #[test]
    fn ac_001_8_other_popups_go_to_the_browser_or_nowhere() {
        let policy = Policy::default();
        assert_eq!(
            policy.new_window(&url("https://twitter.com/intent/tweet")),
            PopupDecision::OpenExternally
        );
        assert_eq!(
            policy.new_window(&url("file:///etc/hosts")),
            PopupDecision::Ignore
        );
    }

    #[test]
    fn ac_001_9_main_frame_leaves_only_for_browser_urls() {
        let policy = Policy::default();
        assert_eq!(
            policy.main_frame_load(&url("https://example.org/")),
            LoadDecision::LeaveToBrowser
        );
        for raw in [
            "https://www.deezer.com/br/",
            "https://accounts.google.com/signin",
            "about:blank",
        ] {
            assert_eq!(
                policy.main_frame_load(&url(raw)),
                LoadDecision::Stay,
                "{raw}"
            );
        }
    }

    #[test]
    fn ac_001_10_described_urls_keep_scheme_and_host_only() {
        assert_eq!(
            describe(&url(
                "https://accounts.google.com/o/oauth2?code=secret#token=x"
            )),
            "https://accounts.google.com"
        );
        assert_eq!(describe(&url("mailto:someone@example.org")), "mailto:");
    }

    #[test]
    fn extra_app_origin_matches_scheme_host_and_port_exactly() {
        let policy = Policy::default().with_app_origin(&url("http://127.0.0.1:4000/player.html"));
        assert_eq!(
            policy.classify(&url("http://127.0.0.1:4000/other")),
            Destination::App
        );
        assert_eq!(
            policy.classify(&url("http://127.0.0.1:4001/")),
            Destination::Browser
        );
    }

    #[test]
    fn ac_001_11_only_the_close_request_url_closes_a_popup() {
        assert!(is_close_request(&url(CLOSE_REQUEST_URL)));
        assert!(is_close_request(&url(
            "https://deezer-tauri.invalid/close?x=1"
        )));
        for raw in [
            "http://deezer-tauri.invalid/close",
            "https://deezer-tauri.invalid/",
            "https://deezer-tauri.invalid/closed",
            "https://evil.com/close",
            "https://deezer-tauri.invalid.evil.com/close",
            "https://accounts.google.com/close",
        ] {
            assert!(!is_close_request(&url(raw)), "{raw}");
        }
    }

    #[test]
    fn ac_001_11_the_popup_script_sends_close_to_the_close_request_url() {
        assert!(POPUP_SCRIPT.contains(&format!("\"{CLOSE_REQUEST_URL}\"")));
        assert!(POPUP_SCRIPT.contains("window.close = request"));
        assert!(POPUP_SCRIPT.contains("if (!window.opener) { return; }"));
    }

    #[test]
    fn the_start_url_is_the_app() {
        assert_eq!(classify(START_URL), Destination::App);
    }
}
