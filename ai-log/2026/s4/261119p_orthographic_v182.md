# 1.8.2: "正交投影" checkbox (orthographic camera)

Date: 2026-11-19. Project: `lobj-view`. Version 1.8.2 (Cargo.toml 1.8.1 → 1.8.2). Follows [261119o](261119o_self_update_v181.md).

## Context / User request
Add an orthographic-projection checkbox inside the top-left status box, default off.

## Decisions & rationale
- Placed in `#status2` right after 格線. Not persisted in localStorage (user said default off; unlike 格線 which is remembered).
- Two cameras (`perspCamera`, `orthoCamera`), `camera` is a `let`; `setOrthographic` swaps it and sets `controls.object`. OrbitControls in ortho mode zooms via `camera.zoom`, so on ortho→persp the zoom is converted to distance to keep the apparent size.
- Ortho frustum = same visible height as the perspective camera at the target distance (`updateOrthoFrustum`), so toggling does not jump. `fitCamera` resets zoom and updates both cameras' near/far.

## Implementation (files)
`web/index.html`, `web/src/main.ts`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit`, `npm run build`, `cargo check` pass. NOT verified in a real browser.

## Open items
`controls.object` is cast to `PerspectiveCamera` to satisfy three's typings. Head/three-light modes use `camera.position`, which should stay valid in ortho but is untested.
