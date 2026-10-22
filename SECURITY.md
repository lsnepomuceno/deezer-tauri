# Security policy

## Supported versions

Only the latest release receives fixes.

## Reporting a vulnerability

Please do not open a public issue. Report it privately through [GitHub's security advisories](https://github.com/lsnepomuceno/deezer-tauri/security/advisories/new), with the version, the platform and the steps to reproduce.

You will get an answer within a week. A confirmed issue is fixed in a new release and credited in the advisory, unless you prefer otherwise.

## What is in scope

The shell runs a remote page, so its boundary is what that page can reach. In scope:

- Anything that lets a web page (Deezer's or one it navigates to) call a command other than `report_media`, run code in the shell process, or read or write files.
- A way around the navigation rules of `specs/features/001-webview-shell` that keeps an untrusted page inside the app window.
- Text from the page reaching a notification, a menu or a log without the sanitizing of `specs/features/002-media-bridge`.
- Tokens or URL paths of login flows written to the logs.

Out of scope: Deezer's own website and services, and vulnerabilities of the web engines themselves (report those to WebKitGTK, Microsoft or Apple).
