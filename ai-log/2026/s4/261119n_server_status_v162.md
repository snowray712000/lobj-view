# 1.6.2: server status row (people/tabs, CPU, memory, GPU)

Date: 2026-11-19. Project: `lobj-view`. Version 1.6.2 (1.6.1 → 1.6.2). Follows [261119m](261119m_zip_as_folder_v161.md).

## User request
Show how many "people-tabs" use the server, plus CPU / memory / GPU status, as a new row in the top-left server status; backend samples periodically and caches so it is cheap. Tell them if risky.

## Decisions
- `src/stats.rs`: sampler thread every 3 s (sysinfo 0.40, `System::new()` returns Result), cached; `/api/stats` reads cache only; sampling pauses if nobody read for 20 s. Frontend polls ~5 s (1.5 s until first sample), paused when the tab is hidden.
- People = distinct peer IP (ConnectInfo on the SSE connection, `Shared.peers: HashMap<IpAddr, count>`); ssh -L users all appear as 127.0.0.1 → counted as one. Accepted limitation.
- GPU: only via `nvidia-smi` (every 6 s, CREATE_NO_WINDOW on Windows, disabled after first failure). Not shown on machines without it (this dev PC).
- Caveat told to user: server GPU ≠ GPU doing the 3D rendering (browser's machine); tooltip says so.
- Colours: ≥70% warn, ≥90% err.

## Verification
`cargo test` 10 pass (new `gpu_line`). In the in-app browser: row shows "1 人 / 1 分頁 · CPU 27% · 記憶體 15.0/23.8 GB". GPU path and multi-person counting not exercised manually.

## Open items
AMD/Intel GPU not supported; multi-person count untested with real LAN clients.

## Gotcha
Editing Cargo.toml with PowerShell `Get-Content -Raw | Set-Content` (no `-Encoding utf8` on read) turned its Chinese comments into mojibake, committed in d90b95f and repaired in d01eebb (restored from the previous commit, then re-applied version + sysinfo). Windows PowerShell 5.1 also writes a BOM with `-Encoding utf8`. Use a Python script file (`encoding='utf-8', newline=''`) for edits to files with non-ASCII text; Python code piped through a shell heredoc also gets its non-ASCII literals garbled.
