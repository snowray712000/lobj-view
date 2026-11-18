# ai-log index

One entry per session log. Paths are relative to this file.

- [2026/s4/261118a_lobj_view_scaffold.md](2026/s4/261118a_lobj_view_scaffold.md) — Initial scaffold of lobj-view: an axum + rust-embed server (auto port fallback, SPA fallback, `/api/health`) and a Vite + three.js WebGPURenderer frontend with a fake per-vertex-label demo. Also the design advice on secure context, cargo install + dist, and lobj format alignment.
  Keywords: lobj, lobj-view, scaffold, axum, rust-embed, allow_missing, clap, port fallback, AddrInUse, SPA fallback, Cache-Control, Vite proxy, LOBJ_API, three/webgpu, WebGPURenderer, WebGL2 fallback, TSL, secure context, ssh -L, cargo install include web/dist, ResizeObserver, "texture size … is empty", depthBuffer, swapchain size 0, tsc process @types/node, Windows
- [2026/s4/261118b_release_build_and_linux_cross.md](2026/s4/261118b_release_build_and_linux_cross.md) — Built the single-exe release (rust-embed, no code change) and cross-compiled a static Linux musl binary from Windows with rust-lld; verified on Windows and WSL Ubuntu. Discussed GitHub Actions as an alternative (not implemented).
  Keywords: release build, single exe, rust-embed, x86_64-unknown-linux-musl, rust-lld, CARGO_TARGET_*_LINKER, static-pie, cross compile, WSL Ubuntu, GitHub Actions, --host 0.0.0.0, WebGPU secure context, ssh -L, Windows
