//! 002 AC-12: the real binary, in the platform's real engine, against a local page that
//! behaves like the player. Run with `cargo test --features e2e --test e2e`.
#![cfg(feature = "e2e")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const PAGE: &str = include_str!("fixtures/player.html");

fn serve_fixture() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap_or(0);
            let found = request[..read].starts_with(b"GET /player.html ");
            let (status, body) = if found {
                ("200 OK", PAGE)
            } else {
                ("404 Not Found", "")
            };
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    port
}

#[test]
fn ac_002_12_the_real_engine_reports_to_and_obeys_the_shell() {
    let port = serve_fixture();
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings.json");
    let report = dir.path().join("report.json");
    std::fs::write(&settings, r#"{"notifications": false}"#).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_deezer-tauri"))
        .env(
            "DEEZER_TAURI_E2E_URL",
            format!("http://127.0.0.1:{port}/player.html"),
        )
        .env("DEEZER_TAURI_E2E_SETTINGS", &settings)
        .env("DEEZER_TAURI_E2E_REPORT", &report)
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the app did not finish the end-to-end run in 120 s");
        }
        thread::sleep(Duration::from_millis(200));
    };

    let raw = std::fs::read_to_string(&report)
        .unwrap_or_else(|_| panic!("the app exited with {status} and wrote no report"));
    let steps: Vec<serde_json::Value> = serde_json::from_str(&raw).unwrap();
    assert!(status.success(), "end-to-end run failed ({status}):\n{raw}");
    assert_eq!(steps.len(), 14, "unexpected number of steps:\n{raw}");
    assert!(steps.iter().all(|step| step["ok"] == true), "{raw}");
}
