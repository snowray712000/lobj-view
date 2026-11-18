# 261118b — Single-exe release build, Linux (musl) cross-compile from Windows

Date: 2026-11-18
Project root: `D:\coding\ccu-cadcam\rust-workspace\lobj-view\`
Follows: [261118a](261118a_lobj_view_scaffold.md)

## Context / User request

- User wanted to try the "one exe, just run it" mode with rust-embed (the scaffold from 261118a already embedded `web/dist` via `#[derive(RustEmbed)] #[folder = "web/dist/"]` in `src/main.rs`; no code change was needed).
- After that worked on Windows ("很酷"), user asked for an Ubuntu build to copy over and run.
- Mid-task the user asked whether GitHub (Actions) could build it instead of a local Windows cross-compile.
- Final user feedback: it works on Linux when used through an SSH tunnel; and on Windows, `--host 0.0.0.0` makes the browser lose WebGPU (non-secure context over LAN IP http), which the user considered the expected/good behavior (confirms the secure-context note in 261118a). User stopped there.

## Decisions & rationale

1. **No code changes.** Only builds. rust-embed embeds in release only; in debug it reads `web/dist` from disk at runtime.
2. **Linux build via `x86_64-unknown-linux-musl` + `rust-lld`, cross-compiled on Windows.**
   - WSL Ubuntu 24.04 existed but had no cargo and no gcc (installing would need sudo), so building inside WSL was rejected.
   - All dependencies are pure Rust (axum, tokio, clap, rust-embed, mime_guess, anyhow), so no C toolchain is needed.
   - musl gives a static-pie binary independent of the target's glibc version (a build on WSL would have required glibc >= 2.39).
3. **GitHub Actions** was explained as the better path for multi-platform builds, tagged releases, or future C dependencies (e.g. openssl). Not implemented: the repo has no remote, and nothing is pushed without the user's action. Offered to write `.github/workflows/release.yml` (needs `npm ci && npm run build` before `cargo build --release`, or reuse committed `web/dist`). User did not take it up in this session.

## Implementation (commands)

```
cargo build --release
  -> target/release/lobj-view.exe  (~2.6 MB)

rustup target add x86_64-unknown-linux-musl
CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld \
  cargo build --release --target x86_64-unknown-linux-musl
  -> target/x86_64-unknown-linux-musl/release/lobj-view  (~3.7 MB, static-pie, not stripped)
```
The linker is set only through the env var (no `.cargo/config.toml` was created). On the target: `chmod +x lobj-view && ./lobj-view`.

## Pitfalls & gotchas

- Debug builds do not embed; always test the single-exe claim with `--release`.
- After editing the frontend, run `npm run build` in `web/` before `cargo build --release`, or the exe contains the stale front end.
- Default bind is 127.0.0.1; remote use needs `ssh -L` (WebGPU needs a secure context), `--host 0.0.0.0` loses WebGPU (falls back to WebGL2 / unavailable) and has no auth.

## Verification

- Windows exe copied to a folder without `web/` and run with `--port 18080`: `/` 200 (index.html), hashed JS asset 200 `text/javascript` (~842 KB), `/api/health` -> `ok`.
- Linux binary run inside WSL Ubuntu 24.04: `file` reports ELF 64-bit static-pie; `/` 200, `/api/health` -> `ok`.
- User confirmed on their own: Linux target works via SSH tunnel; Windows `--host 0.0.0.0` loses WebGPU.
- NOT verified by me: rendering in a real browser; other distros/architectures (aarch64 not built).

## Open items

- Optional GitHub Actions release workflow (no remote yet).
- Optionally add `strip`/`lto`/`opt-level` release profile to shrink the binary (not tried).
