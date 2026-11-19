# 1.8.1: self-update from GitHub Releases (check → download → replace → restart)

Date: 2026-11-19. Project: `lobj-view`. Version 1.8.1 (1.7.1 → 1.8.1, user asked for 1.8.1 directly). Follows [261119n](261119n_server_status_v162.md).

## User request
Can lobj-view have a "check for updates" button in the UI that, when GitHub has a newer release, updates and restarts? If possible, implement it as 1.8.1.

## Decisions
- New `src/update.rs`. `GET /api/update` (cached 10 min, `?force=1` bypasses; unauthenticated GitHub API is 60/h) and `POST /api/update`. Also CLI `lobj-view update [--check]`.
- POST uses the same guard as file association (`assoc::guard_with`, refactored from `guard`): local-only listener, localhost Host header, `X-Lobj-Requested: 1`. With `--host 0.0.0.0` the web page refuses and points to the CLI.
- HTTP: `ureq` 3 (rustls + ring; confirmed no aws-lc-rs in Cargo.lock). Asset chosen by compile-time target from build.rs (`LOBJ_VIEW_TARGET`), linux-gnu mapped to the musl asset. Names follow release.yml: `lobj-view-v{ver}-{target}.zip|tar.gz`.
- Safety: check size, check GitHub asset `digest` (sha256) when present, run `<new> --version` and require the version string before `self_replace`. Any failure leaves the old exe untouched.
- Restart: main `select!` also waits on `update::restart_requested()` (a `LazyLock<Notify>`, fired 500 ms after the POST response). Then drop `Registration` (removes instance.txt) and relaunch with the same args plus env `LOBJ_VIEW_RESTART=<pid>:<port>`:
  - Unix: `exec` (same pid, terminal Ctrl+C still works; sockets are CLOEXEC), pid sent as 0 = don't wait.
  - Windows: spawn child, parent `exit(0)`; child waits for the parent pid (sysinfo) before `instance::claim`, so a PATH argument isn't forwarded to the dying instance.
  - New process reuses the port (`bind_retry`, 10 s) and never opens a browser.
- Exe path captured at startup (`remember_exe`) because Linux `current_exe()` becomes "… (deleted)" after replacement.
- Frontend: button next to the version; silent check on page load turns it green "⬆ 有新版 vX"; dialog shows current/latest, GitHub link, apply button; after apply it polls `/api/version` until the new version answers, then reloads.

## Verification
- `cargo test update` 2 pass (version parsing, release JSON → asset/sha256).
- End-to-end on Windows: built a copy with version 1.6.9, ran `--new --port 18080`; GET showed `canApply:true`, POST without header → 403, POST with header → `{"version":"1.7.1"}`; ~2 s later the same port answered 1.7.1 and the copied exe's `--version` printed 1.7.1. Real GitHub v1.7.1 asset was downloaded and digest-verified.
- In-app browser on 1.8.1: button renders, dialog shows "目前版本 v1.8.1，GitHub 最新版 v1.7.1 / 已是最新版".
- Not exercised: Linux/macOS `exec` path and tar.gz extraction against a real asset (unit-level only).

## Follow-up: `--allow-remote-update`
User asked for an explicit opt-in so LAN pages (with `--host 0.0.0.0`) can trigger the update; default stays off. `update::guard` (static `ALLOW_REMOTE`, set from main) skips the local-only and Host checks when on, but still requires `X-Lobj-Requested` (CSRF). A startup notice is printed when it's on with a non-loopback host; relaunch reuses args, so the flag survives restarts. Tested with `--host 0.0.0.0 --http`: without the flag POST → 403 with a message mentioning the flag; with it → passes guard ("已是最新版"), and missing header → 403.

## Open items
- Real test of 1.8.1 → next release only possible after a newer release exists (tracked in doc/TODO.md).
- Linux/macOS `exec` restart and tar.gz extraction not verified on a real machine.
- Other open tabs lose their loaded models on restart (page reload); acceptable.
- Not yet tagged/released: pushing tag `v1.8.1` will trigger the release workflow.

## Note
Non-ASCII edits this session were done with Python scripts (`encoding='utf-8', newline=''`) passed through the Bash tool's heredoc; Chinese text came out intact (checked with grep and `git diff`), unlike the PowerShell mishap noted in 261119n.
