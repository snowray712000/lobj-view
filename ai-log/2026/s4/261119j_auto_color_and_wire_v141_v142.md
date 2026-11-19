# 1.4.1–1.4.2: auto color for unlabeled meshes, wire checkbox

Date: 2026-11-19. Project: `lobj-view`. Versions 1.4.1 and 1.4.2 (Cargo.toml 1.3.1 → 1.4.2). Follows [261119i](261119i_open_with_list_v126.md).

## Context / User request
- 1.4.1: after loading, a mesh without labels should get an arbitrary color, preferably one not yet used; repeats are fine once there are more meshes than colors. Then refined: files that already have colors (.ply/.obj) use mix 0.2; labeled+colored 0.6; unlabeled+uncolored 0.6.
- 1.4.2: per-object "wire" checkbox showing the mesh edges as black lines.

## Decisions & rationale
- "Unlabeled" = `labels` null or all `< 0` (-1 = unknown). Auto color is picked once per file (`autoColored` set) so later manual choices survive deselect/reselect.
- `pickAutoColor` counts colors used by the other loaded meshes and takes the least used (first in palette order on ties) → unused first, even spread after exhaustion. Gray is excluded from the auto palette (looks like the "uncertain" gray).
- The frontend cannot tell file colors from backend-filled colors (backend fills colors from labels, and all-(-1) labels for stl/obj/ply). So the backend now sends response header `X-Lobj-File-Colors: 1|0` (colors present before filling); `fetchLobj` stores it as `LobjData.fileHasColors` (missing header → treated as true).
- Wire: `WireframeGeometry` + `LineBasicMaterial` black `LineSegments`, created lazily on first check, child of the mesh (`mesh.userData.wire`), opacity follows alpha; face material gets `polygonOffset` while wire is on to avoid z-fighting; `disposeMesh` frees the wire.

## Implementation (files)
`web/src/props.ts` (`AUTO_PALETTE`, `pickAutoColor`, `ObjProps.wire`, wire in `applyProps`), `web/src/main.ts` (`Loaded.unlabeled/fileHasColors`, auto color on load + `renderPanel()`, wire checkbox, dispose), `web/src/lobj.ts`, `src/lobj_api.rs` (`encode` returns `(bytes, file_has_colors)`, header), `Cargo.toml`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
- `tsc --noEmit`, `npm run build`, `cargo check` pass for both versions.
- NOT verified in a real browser: auto color visuals, wire rendering on WebGPU vs WebGL2, wire cost on dense dental meshes.

## Open items
- Wire on very dense meshes may be heavy/dark; consider edge-only (angle threshold) lines.
