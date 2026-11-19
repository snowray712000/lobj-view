# 1.8.3: "單面" checkbox (front-face-only rendering)

Date: 2026-11-19. Project: `lobj-view`. Version 1.8.3 (Cargo.toml 1.8.2 → 1.8.3). Follows [261119p](261119p_orthographic_v182.md).

## Context / User request
User asked whether meshes could be drawn single-sided (they believed they were currently drawn double-sided — confirmed: material was `THREE.DoubleSide`).

## Decisions & rationale
- Implemented as a status-box checkbox "單面", default OFF so existing behaviour (double-sided) is unchanged; the request was a "can we", so an option is the conservative reading. Not persisted. Easy to flip the default if the user wants single-sided as the norm.
- `FrontSide` = counter-clockwise triangles; models with inconsistent winding will show holes — that is also a useful diagnostic.

## Implementation (files)
`web/index.html`, `web/src/main.ts` (`singleSided`, `meshSide()`, handler updating `material.side`), `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`npm run build` passes. NOT verified in a browser; unclear whether `needsUpdate` is strictly required for `side` changes on the WebGPU renderer.

## Open items
Whether single-sided should be the default; flipped normals on files with reversed winding are not auto-fixed.
