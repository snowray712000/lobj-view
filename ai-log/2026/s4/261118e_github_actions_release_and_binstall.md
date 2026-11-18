# GitHub Actions cross-compile release + cargo-binstall (0.0.3)

## Context / User request
- Project: `lobj-view` (axum + rust-embed server, three.js frontend; `web/dist` is committed so no Node.js is needed to build). Previous logs: 261118b (Windows -> Linux musl cross with rust-lld worked), 261118c (adding TLS via rustls/`ring` broke that cross build: no C compiler on Windows).
- User had never done cross-compile on GitHub and wanted to do it this time ("我沒有在 github 作過 cross-compile，這次就是要作這個").
- Later the user asked for: (a) a doc explaining the GitHub cross-compile procedure, (b) README text on installing via cargo and binstall, (c) a small tutorial on installing the latest cargo (without `apt install cargo`) and then the latest binstall, (d) release version "0.0.0.3" (interpreted as `0.0.3`, since Cargo needs semver; uncertain this was the intent).
- Key goals stated by the user: 1) can cross-compile on GitHub, 2) can install through `cargo binstall`, 3) server running on Ubuntu works from Windows and Mac browsers over HTTPS.

## Decisions & rationale
- Workflow `.github/workflows/release.yml`, triggers: tag `v*` and manual `workflow_dispatch` (manual run builds only; the `release` job is gated on `refs/tags/v*`).
- Linux targets `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` use `cross` (installed with `taiki-e/install-action`) because `ring` needs a C compiler. Windows (`x86_64-pc-windows-msvc`) and macOS (`x86_64-apple-darwin`, `aarch64-apple-darwin`, both on `macos-latest` arm64 runner) are built natively with `cargo build`.
- Packaging: `lobj-view-<ref_name>-<target>/` containing the binary + README.md; `.zip` (7z) for Windows, `.tar.gz` otherwise. Artifacts uploaded per target, then `softprops/action-gh-release@v2` attaches all with generated notes. `permissions: contents: write`.
- binstall metadata in `Cargo.toml` (also added `repository = "https://github.com/snowray712000/lobj-view"`):
  - `pkg-url = "{ repo }/releases/download/v{ version }/{ name }-v{ version }-{ target }.{ archive-format }"`
  - `bin-dir = "{ name }-v{ version }-{ target }/{ bin }{ binary-ext }"`, `pkg-fmt = "tgz"`, override `pkg-fmt = "zip"` for `x86_64-pc-windows-msvc`.
  - Tag must be `v<Cargo.toml version>` because the URL uses `v{ version }`.
- Rejected: `apt install cargo` (too old for edition 2024, conflicts with rustup) -> use rustup. Rejected: cross-compiling from Windows with zig/cc (CI is simpler).

## Implementation (files)
- Added: `.github/workflows/release.yml`, `doc/github-cross-compile.md`, `doc/ubuntu-install-rust-and-binstall.md`.
- Modified: `Cargo.toml` (repository, binstall metadata, version 0.0.3; `Cargo.lock` updated by build), `README.md` (new "安裝" section with binstall / cargo install --git / manual download; version note; build note now points to the CI doc), `doc/TODO.md`, `doc/Versions.md`.
- Docs are in Traditional Chinese (human audience).

## Pitfalls & gotchas
- `cargo` not found on a fresh Ubuntu: Ubuntu suggests `apt install cargo`; use rustup instead. After install, "Your path is missing ~/.cargo/bin" warning -> add to PATH in `~/.bashrc`.
- Earlier in the chat I said the binstall install script "needs cargo"; that was likely wrong — the official script downloads a prebuilt binary. The tutorial doc therefore presents the script as method A and `cargo install cargo-binstall` as method B. How the user actually got binstall installed is not known.
- binstall reads `Cargo.toml` from the repo/tag, so push before tagging; if the Release file is not found binstall silently falls back to compiling from source.
- Editing README via a non-raw Python string turned `\b` into a backspace in the Windows path; fixed (check for stray control characters when writing backslashes).
- `workflow_dispatch` runs use the branch name as `github.ref_name`; branch names with `/` would break the packaging step (not hit).

## Verification
- Reported by the user: the release exists on GitHub, `cargo binstall` installed it on Ubuntu, and the Ubuntu-hosted server worked over HTTPS from Windows and Mac browsers.
- Locally: `cargo build --offline` OK with version 0.0.3; `cargo metadata` parsed the binstall section.
- NOT verified: the 0.0.3 tag/release itself (user does the tagging; nothing was committed/pushed by me), macOS and Windows release binaries, aarch64 Linux binary, `cargo install --git`.

## Open items
- Tag and push `v0.0.3` after committing; confirm the Release contains all 5 archives and that binstall picks up 0.0.3.
- Test the macOS/Windows/aarch64 artifacts; consider macOS Gatekeeper/signing.
- Verify `cargo install --git` / `cargo package` include `web/dist`.
