# 261118a — lobj-view project scaffold (Rust axum + rust-embed backend, Vite + three.js WebGPU frontend)

Date: 2026-11-18
Project root: `D:\coding\ccu-cadcam\rust-workspace\lobj-view\` (the user had pre-created this empty folder)

## Context

- `lobj` is the user's own 3D object format: like `.obj` (stores 3D objects) but **binary** and with a **per-vertex label**. The exact binary layout was NOT provided in this session.
- `lobj-view` = a viewer for `.lobj` files.
- Main constraint: the user often works on a remote machine over **SSH with no display**, so a native wgpu window is not usable. Rendering must happen in a web browser on the user's local machine.
- Future distribution: `cargo install` on another (remote) machine, then open it in a browser "on this computer" (user's words: "會在同網域下" — same network/domain).

## User request (paraphrased)

Phase 1 — the user posted a plan and asked for affirmations and suggestions:
- Frontend: three.js + `.ts` `.html` `.css`. Reason: three.js can use WebGPU; the main reason is that the remote SSH session has no display, so use the web instead of wgpu.
- Package with `rust-embed`; install with cargo on another machine and use from a browser on this machine.
- If the port is taken, automatically pick a free one (and print it).
- The backend initially only serves the frontend; APIs will be added later.
- Frontend built with `npm run build` into `dist`.

Phase 2 — "好，架架看。" (= "OK, scaffold it").

## Decisions & rationale (advice given in phase 1, then applied)

1. **WebGPU requires a secure context.** `navigator.gpu` is only exposed on `https://` or `localhost`/`127.0.0.1`. `http://192.168.x.x:port` gets no WebGPU. Recommended remote workflow: bind to `127.0.0.1` and use `ssh -L <port>:localhost:<port> <host>`, then open `http://localhost:<port>` locally. (Forked side sessions, not this log's main thread, went further on this: the GPU used is always the browser machine's GPU, and mkcert/HTTPS for LAN access. Details live in those forks only; uncertain whether anything was implemented there.)
2. three.js `WebGPURenderer` (`three/webgpu`) falls back to WebGL2 automatically. **GLSL `ShaderMaterial` does not work with WebGPURenderer**, so custom shaders must be written in TSL from the start.
3. **`cargo install` will not run `npm run build`.** rust-embed reads `web/dist` at compile time, so the built `dist` must be shipped inside the crate. Rejected: a `build.rs` that calls npm, because it would force Node.js onto the install machine. Applied: `include = ["src/**", "web/dist/**", "Cargo.toml"]` in `Cargo.toml`, and `web/dist` is intentionally NOT gitignored.
4. Default bind is `127.0.0.1`. `--host 0.0.0.0` is opt-in (no auth, so anyone on the LAN could reach it).
5. Port: `--port` gives the preferred port (keeps SSH tunnels stable); fallback scan upward.
6. Dev workflow: Vite dev server with HMR + `server.proxy` sending `/api` to the Rust backend.
7. Advice for the future lobj format (not implemented): magic + version header; little-endian; 4-byte-aligned sections so the frontend can do zero-copy `new Float32Array(buffer, offset, n)`; labels as `u32` per vertex used as a vertex attribute (color/picking) plus a string table for label names; serve raw bytes as `application/octet-stream`.
8. Suggested a separate `lobj` crate (format read/write, reusable, possibly wasm later). **Deferred**: the user's pre-created folder was `lobj-view/` and the format spec is unknown, so the scaffold is a single crate for now.
9. Future idea (not implemented): CLI `lobj-view file.lobj` + file watching (`notify`) + SSE/WebSocket live reload.

## Implementation

Toolchain seen: cargo/rustc 1.99.1, node v24.12.0, npm 10.8.1, Windows 10.
Deps (added via `cargo add`/`npm install`, latest at the time): axum 0.8.9, tokio 1.54.1 (full), rust-embed 8.14.0, mime_guess 2.0.5, clap 4.6.8 (derive), anyhow 1.0.104; three ^0.187.0, @types/three ^0.187.0, vite ^8.3.8, typescript ^7.0.2.

```
lobj-view/
├─ Cargo.toml        # include web/dist; edition 2024
├─ .gitignore        # /target, /web/node_modules (dist deliberately kept)
├─ src/main.rs
└─ web/
   ├─ package.json   # scripts: dev=vite, build="tsc --noEmit && vite build"
   ├─ tsconfig.json  # include ["src"] only
   ├─ vite.config.ts
   ├─ index.html     # #viewport + #hud (backend / health spans)
   └─ src/main.ts, src/style.css
```

### `src/main.rs`
- `#[derive(RustEmbed)] #[folder = "web/dist/"] #[allow_missing = true] struct Assets;`. In a debug build rust-embed reads from disk at runtime (rebuilding the frontend needs no cargo rebuild). In a release build the files are embedded.
- CLI (`clap`): `--host <IpAddr>` default `127.0.0.1`; `-p/--port <u16>` default `8080`.
- `bind()`: tries `port..=port+50` (`PORT_SEARCH_RANGE = 50`) and skips `AddrInUse`/`PermissionDenied`, then falls back to port 0 (OS-assigned). If the port changed, prints `port X 已被佔用，改用 Y`.
- `print_urls()`: loopback → prints `http://localhost:<port>` plus an `ssh -L` hint. Unspecified (0.0.0.0) → warns that LAN-IP http gets no WebGPU.
- Routes: `GET /api/health` → `"ok"`; `fallback(static_handler)`.
- `static_handler`: `/` → `index.html`; an embedded file is served if it exists; unknown `api/*` → 404 (so the frontend never parses HTML as data); otherwise SPA fallback to `index.html`. If `dist` is missing → 404 with the text "前端尚未 build：請先在 web/ 執行 npm install && npm run build".
- `serve_file`: MIME type via `mime_guess`; `assets/*` → `Cache-Control: public, max-age=31536000, immutable` (Vite hashes the filenames); everything else → `no-cache`.

### `web/vite.config.ts`
- `server.proxy: { '/api': process.env.LOBJ_API ?? 'http://127.0.0.1:8080' }`. Set `LOBJ_API` if the backend moved to another port.
- `build.target: 'esnext'` (top-level await for `renderer.init()`), `chunkSizeWarningLimit: 1500` (three.js alone is about 840 kB minified).

### `web/src/main.ts`
- `import * as THREE from 'three/webgpu'`; `OrbitControls` from `three/addons/controls/OrbitControls.js`.
- `new THREE.WebGPURenderer({antialias:true})`, `await renderer.init()`; backend detected via `(renderer.backend as any).isWebGPUBackend`. The HUD shows `WebGPU` or `WebGL2 (fallback)`, plus a non-secure-context hint when `!window.isSecureContext`.
- Scene: hemisphere + directional light, GridHelper, AxesHelper, and a demo TorusKnot with **fake labels** (`Uint32Array`, 5 groups by x coordinate) mapped to HSL vertex colors, `MeshStandardMaterial({vertexColors:true})`. This placeholder is meant to be replaced by real `.lobj` loading.
- Sizing via `ResizeObserver` on `#viewport`; rendering is skipped while the size is 0 (see pitfalls).
- `fetch('/api/health')` → HUD `server: ok` or `無法連線 (...)`.

## Pitfalls & gotchas

- `tsc` failed on `vite.config.ts` (`Cannot find name 'process'`, no @types/node). Fix: removed `vite.config.ts` from tsconfig `include` (Vite handles the config itself) instead of adding @types/node.
- **WebGPU errors on zero-size canvas**: "The texture size ([Extent3D width:0, height:0 …]) is empty … depthBuffer", "Could not create a swapchain texture of size 0". Cause: rendering began while the page/pane had size 0 (a background tab). Fix: a ResizeObserver plus a `hasSize` guard in `setAnimationLoop`. Note: the browser pane's console log persisted across same-tab reloads, which looked like the error still happened. A **fresh tab** confirmed zero console messages.
- On this dev machine, ports 8080 and 8081 were already taken by other software (8080 returned `{"error":"Invalid session"}`), so the server landed on 8082. Don't assume 8080 is lobj-view.
- Git Bash rewrites URL paths like `/api/health` in echo output (MSYS path conversion). This affected only the display, not the requests.

## Verification

- `cargo check` / `cargo build` (debug) succeeded. `npm run build` succeeded.
- Ran two instances at the same time: they got ports 8082 and 8083 (fallback works).
- curl on 8082: `/api/health` → 200 `ok`; `/` → 200 text/html; `/api/nope` → 404; `/some/route` → 200 text/html (SPA fallback).
- In-app browser (Chromium-based, local GPU): HUD showed `renderer: WebGPU`, `server: ok`; the torus rendered with label colors; a fresh tab had no console messages.
- NOT tested: release build / real embedding, `cargo install`/`cargo package` with `include`, `--host 0.0.0.0`, remote use over `ssh -L`, the WebGL2 fallback path, `npm run dev` + proxy, Linux/macOS.
- Test server processes were stopped afterwards. No git repo exists (nothing committed).

## Open items

- The lobj binary format spec is needed from the user (header, section layout, label type). Then: a parser (Rust and/or TS), an API to serve a file, and loading it in the frontend.
- Possibly split out a `lobj` crate once the format is defined.
- CLI file argument, file watching + live reload.
- `git init` (the project is not under version control).
- Verify a release build and `cargo install` from a packaged crate.
