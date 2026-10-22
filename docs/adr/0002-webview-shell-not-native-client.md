# 0002: A shell around the web player, not a native client

- Status: accepted
- Date: 2026-10-22

## Context

The goal is to listen to Deezer from a lightweight, maintained desktop app on Linux, Windows and macOS. Two designs were considered.

1. A native client that logs in through Deezer's private gateway, asks `media.deezer.com/v1/get_url` for each track and plays the file itself. This is how the official app's web player works internally (read from its bundle, version 5.30.220): the license token, the track tokens and the `BF_CBC_STRIPE` stream all flow through the client, and the stream is decrypted in the client before it reaches `MediaSource`. A native client would have to reproduce that decryption outside the official player.
2. A window around the official web player, with the desktop integration a browser tab lacks.

## Decision

Option 2. The shell loads `https://www.deezer.com` in the platform's web engine and never requests, decrypts or stores audio.

## Consequences

- Reproducing the stream decryption would circumvent the protection Deezer puts on its catalogue and breach its terms of use, with the account at risk. The shell stays clear of that: it is a browser with a narrower purpose.
- Playback, quality, offline limits and every feature of the player are exactly what Deezer ships on the web, and they keep working when Deezer changes its internals.
- The project depends on the web player running in each engine. WebKitGTK was verified on 2026-10-22 (see `docs/architecture.md`); WebView2 is Chromium, which Deezer supports; WKWebView is Safari's engine, which Deezer supports.
- The app is unofficial and has no affiliation with Deezer. It does not use Deezer's logo.
