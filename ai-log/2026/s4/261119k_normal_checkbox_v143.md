# 1.4.3: per-object "normal" checkbox (vertex normals)

Date: 2026-11-19. Project: `lobj-view`. Version 1.4.3 (Cargo.toml 1.4.2 → 1.4.3). Follows [261119j](261119j_auto_color_and_wire_v141_v142.md).

## Context / User request
Add a "normal" checkbox to the object card that shows the vertex normals.

## Decisions & rationale
- One short line per vertex (`LineSegments`, position → position + normal × len), blue (`0x0a84ff`) so it differs from the black wire. `len` = 3% of the geometry's bounding-sphere radius so it scales with the model; the numbers are my choice, not the user's.
- Built lazily on first check, stored in `mesh.userData.normals`, child of the mesh; opacity follows alpha like the wire. `disposeMesh` now loops over `['wire','normals']`.
- Files without normals (.stl/.obj/.ply) already get `computeVertexNormals()` in `buildMesh`, so the lines show those computed normals.

## Implementation (files)
`web/src/props.ts` (`ObjProps.normal`, `buildNormalLines`, `applyProps`), `web/src/main.ts` (checkbox, dispose), `Cargo.toml`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit`, `npm run build`, `cargo check` pass. NOT verified in a real browser; cost on dense dental meshes (one line per vertex) is unmeasured.

## Open items
Line length/color are not user-adjustable; could add a length slider or subsample vertices for dense meshes.
