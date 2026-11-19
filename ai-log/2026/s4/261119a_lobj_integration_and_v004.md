# 261119a — Integrate .lobj reading into lobj-view (backend API + frontend), release 0.0.4

## Context / User request
User (Traditional Chinese) had already finished a Rust `.lobj` reader/writer in a separate learning project
(`D:\coding\tech-rust`, GPUI app, `src/lobj/{mod,format,labels,colors}.rs`, plan in its `doc/plan-lobj.md`).
Request: "integrate the finished lobj into this workspace" (`D:\coding\ccu-cadcam\rust-workspace`, which contained
`lobj-view` = axum + Vite/three.js viewer, and `case-test/` = sample data cloned in, **not** to be pushed to GitHub).
Frontend too ("front end can just mimic the back end"). Front/back transport = **binary stream in the .lobj format**;
inside Rust / three.js each side uses its own native structures. What the Rust backend later does with the mesh
(CAD/CAM processing) is undecided; results should eventually be converted back and sent to the frontend.
Scope for this session: only get .lobj *reading* integrated.
Afterwards the user reported the file dropdown did nothing (fixed), confirmed it worked, and asked to release as 0.0.4.

## .lobj format (from tech-rust plan, verified there against case-test files)
Little-endian, no magic/version. `i32 x7 counts=[nV,nNormal,nColor,nLabel,nTex2d,nTri,nTexId]`, then
`f32 vertices(nV*3)`, `f32 normals`, `f32 colors`, `i8 labels`, `f32 tex2d (nTex2d*2)`, `u32 triangles(nTri*3)`, `u32 texId`.
Total length must equal exactly 28 + sum(count*itemSize). `.lobj` labels = legacy arch_index (needs jaw to convert);
`.lobj2` labels = FDI. In-memory canonical label scheme = FDI (-1 uncertain, 0 gingiva, 11–48 permanent, 51–85 primary, 100 bracket).

## Decisions & rationale
- **Shared crate `rust-workspace/lobj/`** (std-only, copy of tech-rust `src/lobj/*`; `mod.rs` -> `lib.rs`), used by
  `lobj-view` via `lobj = { path = "../lobj" }`. Not a Cargo workspace, because lobj-view is meant to stay a standalone repo with its own
  Cargo.lock/.github. tech-rust was left untouched (so there are now two copies).
- Wire format = the same .lobj layout, but the backend normalizes: labels always FDI (legacy `.lobj` converted server-side, so the
  frontend never sees arch_index), and if the file has no colors the backend fills `colors` from labels with `Palette::Unique`
  (single palette source in Rust).
- Only files directly in `--dir` are served; names containing `/ \ :` or starting with `.` or lacking .lobj/.lobj2 extension -> 400.
- Legacy `.lobj` whose filename has no `upper`/`lower` needs `?jaw=upper|lower` else 422 (`JawRequired`); frontend has a jaw select.

## Rejected alternatives
- Making rust-workspace a Cargo workspace (breaks lobj-view's standalone repo/CI layout).
- Arbitrary path parameter for the API (arbitrary file read). Frontend palette duplicate (kept in Rust only).
- Drag&drop local file parsing in the browser (not needed since files live on the server; also would need jaw/palette in TS).

## Implementation
- `lobj/`: `Cargo.toml` (edition 2024), `src/{lib,format,labels,colors}.rs`, `tests/case_test_files.rs` (reads `../case-test`, skips if missing).
  One edition-2024 fix: closure pattern `|&(&before, &after)|` in `save()`.
- `lobj-view/src/lobj_api.rs` (new): `router(dir)`; `GET /api/lobj` -> JSON `[{name,size}]`; `GET /api/lobj/{name}[?jaw=]` ->
  `application/octet-stream` via `lobj::load` + fill colors + `format::to_bytes`; blocking work in `spawn_blocking`. Merged in `main.rs`; new CLI `-d/--dir` (default `.`).
- `lobj-view/web/src/lobj.ts` (new): `parseLobj(ArrayBuffer)` (validates length/counts/triangle indices; uses `slice` copies because
  segment offsets after the i8 labels are not 4-byte aligned), `labelCounts`, `listLobj`, `fetchLobj`.
- `main.ts`: removed the torus-knot demo; `showLobj` builds BufferGeometry (position, index, normals or computeVertexNormals, color), DoubleSide material,
  fits camera to bounding sphere, shows counts. `index.html`: file select, jaw select, info div. `style.css`: see pitfalls.

## Pitfalls & gotchas
- **User-reported bug**: file dropdown unresponsive because `#hud { pointer-events: none }` in style.css. Fixed with `#hud select { pointer-events: auto }`
  (+ `#info { white-space: pre-line }`). Not caught earlier because only API/build were verified, not the browser.
- Edition 2024 rejects `|(&a, &b)|` closure patterns over `&(T,U)` items.
- On Windows PowerShell, `cargo run` in background via Start-Process failed to start the server within the wait time; build first, then start the exe.
- Console output of Chinese through this PowerShell is mojibake (display only; files are UTF-8).

## Verification
- `lobj`: 20 unit + 4 integration tests (3 real files, byte-exact round trip) pass.
- `lobj-view`: `cargo test` (2 tests), `npm run build` (tsc + vite) OK.
- Server run with `--dir ../case-test`: list OK; upper .lobj2 = 200, 9,118,556 bytes (matches formula incl. added colors);
  legacy .lobj without jaw = 422; with `?jaw=upper` = 200, 2,127,532 bytes; `..\x.lobj` = 400; missing = 404; .stl = 400.
- User confirmed in the browser after the CSS fix that it works. Rendering correctness (colors/camera) was judged by the user only; I did not inspect it.
- Version bumped to 0.0.4 in Cargo.toml. No git tag/push done (project is not a git repo; user said not to use GitHub for case-test).

## Open items
- CI (`.github/workflows/release.yml`) checks out lobj-view alone; the `../lobj` path dependency would fail there (needs same-repo layout or git dependency).
- Two copies of the lobj code (tech-rust vs `lobj/`); make tech-rust use a path dependency.
- Backend -> frontend result path for processed meshes (e.g. POST) not designed yet; depends on the CAD/CAM processing chosen.
- Command-line file argument, file watch/reload still pending (see doc/TODO.md).
