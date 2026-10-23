//! 002 AC-12: the real binary, in the platform's real engine, against a local page that
//! behaves like the player, plus 001 AC-7 to AC-9 in that engine. Run with
//! `cargo test --features e2e --test e2e`. The smoke test against deezer.com needs the
//! network and is ignored unless asked for: `--ignored smoke`.
#![cfg(feature = "e2e")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

const PLAYER: &str = include_str!("fixtures/player.html");
const POPUP: &str = include_str!("fixtures/popup.html");

fn serve_fixture() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap_or(0);
            let request = &request[..read];
            let (status, body) = if request.starts_with(b"GET /player.html ") {
                ("200 OK", PLAYER)
            } else if request.starts_with(b"GET /popup.html ") {
                ("200 OK", POPUP)
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

struct Run {
    status: std::process::ExitStatus,
    report: String,
    steps: Vec<serde_json::Value>,
}

fn run_app(url: &str, smoke: bool) -> Run {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings.json");
    let report = dir.path().join("report.json");
    std::fs::write(&settings, r#"{"notifications": false}"#).unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_deezer-tauri"));
    command
        .env("DEEZER_TAURI_E2E_URL", url)
        .env("DEEZER_TAURI_E2E_SETTINGS", &settings)
        .env("DEEZER_TAURI_E2E_REPORT", &report);
    if smoke {
        command.env("DEEZER_TAURI_E2E_SMOKE", "1");
    }
    let mut child = command.spawn().unwrap();

    let deadline = Instant::now() + Duration::from_secs(120);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("the app did not finish the run in 120 s");
        }
        thread::sleep(Duration::from_millis(200));
    };

    let report = std::fs::read_to_string(&report)
        .unwrap_or_else(|_| panic!("the app exited with {status} and wrote no report"));
    let steps = serde_json::from_str(&report).unwrap();
    Run {
        status,
        report,
        steps,
    }
}

fn names(steps: &[serde_json::Value]) -> Vec<&str> {
    steps
        .iter()
        .filter_map(|step| step["name"].as_str())
        .collect()
}

#[test]
fn ac_002_12_the_real_engine_reports_to_and_obeys_the_shell() {
    let port = serve_fixture();
    let run = run_app(&format!("http://127.0.0.1:{port}/player.html"), false);
    let Run {
        status,
        report,
        steps,
    } = run;
    assert!(
        status.success(),
        "end-to-end run failed ({status}):\n{report}"
    );
    assert!(steps.iter().all(|step| step["ok"] == true), "{report}");

    let names = names(&steps);
    for expected in [
        "the fixture's snapshot reaches Rust",
        "the tray offers to hide it again",
        "mailto opens in the system handler",
        "a page outside Deezer opens in the browser",
        "the window returns to the player",
    ] {
        assert!(
            names.contains(&expected),
            "missing step {expected:?}:\n{report}"
        );
    }
    if cfg!(any(target_os = "linux", target_os = "windows")) {
        for expected in [
            "a popup on an app origin opens in the app, with its opener",
            "the popup is a window of its own, sized as asked",
            "the popup's window.close() closes its window",
            "a popup outside Deezer opens in the browser",
        ] {
            assert!(
                names.contains(&expected),
                "missing step {expected:?}:\n{report}"
            );
        }
    }
}

/// Deezer itself in the platform's engine: the page loads, the bridge reports through
/// the production capability, a preview from Deezer's CDN plays, and (Linux and Windows)
/// the Google button of Deezer's login opens Google in an app popup that closes itself.
/// Needs the network; run with `--ignored smoke`.
#[test]
#[ignore]
fn smoke_deezer_com_loads_and_reports_in_this_engine() {
    let Run {
        status,
        report,
        steps,
    } = run_app("https://www.deezer.com/", true);
    println!("{report}");
    assert!(status.success(), "smoke run failed ({status}):\n{report}");
    assert!(steps.iter().all(|step| step["ok"] == true), "{report}");

    let names = names(&steps);
    let mut expected = vec![
        "deezer.com loads in the engine",
        "the bridge reports from deezer.com through the production capability",
        "the window is still on Deezer",
        "a Deezer preview plays in the engine",
    ];
    if cfg!(any(target_os = "linux", target_os = "windows")) {
        expected.extend([
            "Deezer's login page loads",
            "the login page shows the Google button",
            "Google's sign-in opens in an app popup",
            "Google's popup closes like a sign-in popup does",
        ]);
    }
    for step in expected {
        assert!(names.contains(&step), "missing step {step:?}:\n{report}");
    }
}
