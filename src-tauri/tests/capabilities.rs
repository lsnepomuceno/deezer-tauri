//! 002 AC-9: what the remote page is allowed to call.

use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde_json::Value;
use tauri::utils::acl::RemoteUrlPattern;
use tauri::Url;

fn capabilities() -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let mut found: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let raw = fs::read_to_string(&path).unwrap();
            (
                path.display().to_string(),
                serde_json::from_str(&raw).unwrap(),
            )
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

#[test]
fn ac_002_9_the_only_capability_grants_report_media_to_deezer_in_main() {
    let all = capabilities();
    assert_eq!(all.len(), 1, "{all:?}");
    let (_, remote) = &all[0];
    assert_eq!(remote["identifier"], "deezer-remote");
    assert_eq!(strings(&remote["windows"]), ["main"]);
    assert_eq!(strings(&remote["remote"]["urls"]), ["https://*.deezer.com"]);
    assert_eq!(strings(&remote["permissions"]), ["allow-report-media"]);
    assert!(remote.get("webviews").is_none());
}

#[test]
fn ac_002_9_the_remote_pattern_matches_deezer_and_nothing_else() {
    let pattern = RemoteUrlPattern::from_str("https://*.deezer.com").unwrap();
    let test = |raw: &str| pattern.test(&Url::parse(raw).unwrap());
    for allowed in [
        "https://www.deezer.com/",
        "https://www.deezer.com/br/album/302127",
        "https://account.deezer.com/login?x=1",
    ] {
        assert!(test(allowed), "{allowed}");
    }
    for refused in [
        "http://www.deezer.com/",
        "https://www.deezer.com.evil.net/",
        "https://evil-deezer.com/",
        "https://evil.com/www.deezer.com",
        "https://accounts.google.com/",
    ] {
        assert!(!test(refused), "{refused}");
    }
}
