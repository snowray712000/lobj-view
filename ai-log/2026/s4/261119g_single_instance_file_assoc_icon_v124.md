# 1.2.2–1.2.4: single instance, Windows file association, exe icon + favicon

Date: 2026-11-19. Project: `lobj-view`. Versions 1.2.2, 1.2.3, 1.2.4 (all committed together as 1.2.4).

## Context / User request
The user (Traditional Chinese) wanted "double-click a .lobj/.lobj2/.stl/.ply/.obj → open in lobj-view" after a `cargo binstall` install, starting with Windows 10+:
- If lobj-view is already running → update its dir to the file's folder; if not running → start it (a console window) at that folder.
- Then: do the registration from the Rust backend with a web UI (like `/trust`), choose register/unregister and which extensions.
- Version mapping decided by the user: 1.2.2 = single instance + dynamic dir, 1.2.3 = file association, 1.2.4 = exe icon (from `lobj-view.png`, then a better generated icon), plus favicon for the web page.

## Decisions & rationale
- **cargo-binstall has no post-install hook** → registration is an explicit `register` subcommand / web button (same pattern as `trust --install`).
- **"Rust backend has higher privilege" is not true**: it runs as the same user. HKCU writes need no admin anyway. The real limit is Windows 10 `UserChoice` (hash-protected): a program cannot silently take over an extension that already has a default (here `.stl/.ply/.obj` belong to Store apps). So `register` only adds `OpenWithProgids` (candidate) and sets the class default **only when the extension has no effective default** (`.lobj/.lobj2`); UI/CLI tell the user to pick it in Settings (`ms-settings:defaultapps` button).
- **Single instance IPC is NOT the HTTP API**: a separate listener on `127.0.0.1:<random>` with a token. Protocol `OPEN\t<token>\t<abs path>\n` → `OK\n` / `ERR\t<msg>\n`. Port/token/pid in `<data_local_dir>/lobj-view/instance.txt` (0600 on unix). Reason: an HTTP endpoint that switches the served root would expose arbitrary dirs to the LAN / CSRF / DNS rebinding. Liveness = connect + reply; stale file simply fails to connect. `instance.lock` (create_new, 15 s stale) handles several processes launched at once (Explorer multi-select).
- Root dir is now `Shared.root: RwLock<PathBuf>` read per request. The web page learns about opens through SSE `/api/events` (initial event `seq` 0 = nothing opened → normal `browse('')`; `seq>0` → front end clears selection/loaded meshes and browses to the file). SSE connection count = "a tab is open" → no new browser tab; `open_browser()` is debounced 10 s.
- Web assoc endpoints (`/api/assoc`, `/api/assoc/settings`): only when `--host` is loopback; `Host` must be localhost/127.0.0.1/[::1] (DNS rebinding); POST needs `X-Lobj-Requested: 1` (forces CORS preflight, which the server never answers → CSRF). CLI and web share `assoc.rs`.
- Windows registry: ProgId `lobj-view.file` under `HKCU\Software\Classes` (`shell\open\command` = `"exe" "%1"`, `DefaultIcon` = `exe,0`), `SHChangeNotify(SHCNE_ASSOCCHANGED)` after changes. Unregister removes our values and the ProgId once nothing references it.
- Icon: `build.rs` + `winresource` (needs `rc.exe`; failure only warns). `.ico` is hand-assembled from PNG entries so 16/24 px can use a mesh-free variant (the mesh made 16 px muddy). Design: blue rounded square, white molar with triangle mesh and a few label-coloured triangles. `include` in Cargo.toml gained `build.rs` and the `.ico`. Favicon = same `.ico` in `web/public/favicon.ico`.

## Pitfalls
- Python heredoc edits of Rust sources: `\t` in non-raw strings turned into real tabs; files have mixed CRLF/LF, so string replaces silently missed → normalise line endings in the edit helper and grep for literal tabs.
- clap: optional positional `PATH` + subcommands needs `args_conflicts_with_subcommands = true`.
- Python's `/tmp` is not Git Bash's `/tmp` on Windows; write generated files to the scratchpad.

## Implementation (files)
New: `src/instance.rs`, `src/assoc.rs`, `build.rs`, `assets/lobj-view.{ico,png}`, `web/public/favicon.ico`. Changed: `src/main.rs` (PATH/`--new`/`--no-browser`, `register|unregister|status`, ctrl-c cleanup), `src/lobj_api.rs` (shared root, SSE), `web/*` (SSE handling, "預設程式…" dialog, favicon), `Cargo.toml` (deps `tokio-stream`, `open`, `serde_json`, `winreg`, build-dep `winresource`), `doc/Versions.md`, `README.md`.

## Verification
- `cargo test` 7 passed; debug + release build OK; `npm run build` OK.
- E2E by CLI: forward to running instance, reject `.txt` (exit 1), stale `instance.txt` → becomes server, SSE events seen via curl, assoc API guards (403 without header / bad Host / `--host 0.0.0.0`, 400 bad ext), register/unregister leaves registry clean.
- User confirmed by hand: double-click open works, the exe icon shows.
- NOT verified: web dialog look, favicon in a real tab, GitHub Actions Windows build with `rc.exe`, multi-select open (only the last file stays shown).

## Open items
- 1.2.5: record the previous association before registering so `unregister` can restore it.
- Linux (`.desktop` + `xdg-mime`) and macOS (needs `.app` bundle) not supported.
