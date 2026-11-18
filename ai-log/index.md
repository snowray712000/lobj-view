# ai-log index

One entry per session log. Paths are relative to this file.

- [2026/s4/261118a_lobj_view_scaffold.md](2026/s4/261118a_lobj_view_scaffold.md) — Initial scaffold of lobj-view: an axum + rust-embed server (auto port fallback, SPA fallback, `/api/health`) and a Vite + three.js WebGPURenderer frontend with a fake per-vertex-label demo. Also the design advice on secure context, cargo install + dist, and lobj format alignment.
  Keywords: lobj, lobj-view, scaffold, axum, rust-embed, allow_missing, clap, port fallback, AddrInUse, SPA fallback, Cache-Control, Vite proxy, LOBJ_API, three/webgpu, WebGPURenderer, WebGL2 fallback, TSL, secure context, ssh -L, cargo install include web/dist, ResizeObserver, "texture size … is empty", depthBuffer, swapchain size 0, tsc process @types/node, Windows
- [2026/s4/261118b_release_build_and_linux_cross.md](2026/s4/261118b_release_build_and_linux_cross.md) — Built the single-exe release (rust-embed, no code change) and cross-compiled a static Linux musl binary from Windows with rust-lld; verified on Windows and WSL Ubuntu. Discussed GitHub Actions as an alternative (not implemented).
  Keywords: release build, single exe, rust-embed, x86_64-unknown-linux-musl, rust-lld, CARGO_TARGET_*_LINKER, static-pie, cross compile, WSL Ubuntu, GitHub Actions, --host 0.0.0.0, WebGPU secure context, ssh -L, Windows
- [2026/s4/261118c_https_local_ca.md](2026/s4/261118c_https_local_ca.md) — Added HTTPS so `--host 0.0.0.0` gets a secure context: auto local CA + leaf cert (rcgen, rustls/ring, axum-server), `trust [--install]` subcommand, `/ca.crt` and `/trust` routes. Side effect: musl cross-compile from Windows now fails (ring needs a C compiler).
  Keywords: https, tls, rustls, ring, rcgen, local CA, trust --install, certutil, security add-trusted-cert, SAN, /ca.crt, /trust, --host 0.0.0.0, WebGPU secure context, axum-server, musl cross-compile, ToolNotFound, schannel --ssl-no-revoke, Windows
- [2026/s4/261118d_readme_and_v002.md](2026/s4/261118d_readme_and_v002.md) — User verified HTTPS on Windows and a LAN Mac mini; added macOS "系統 keychain" tip and browser-restart gotcha, wrote README.md, bumped to 0.0.2. Ubuntu-as-server test deferred.
  Keywords: README.md, 0.0.2, macOS keychain 系統, restart browser, trust --install, ca.crt, /trust, Mac mini, Ubuntu server deferred, Cargo.toml readme include
