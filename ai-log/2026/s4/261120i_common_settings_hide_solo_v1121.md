# 1.12.1: Shared settings block + object list with hide/solo

Date: 2026-11-20. Project: `lobj-view`. Version 1.11.1 → 1.12.1.

## Context / User request
Right panel had one card per object, each with its own settings. User wanted the settings in one shared block, and the object list as a table (Excel sketch) with `h` (hide) and `s` (solo) checkboxes. Solo rule: no solo checked → show as configured; any solo checked → non-solo objects with alpha > 0.5 are shown at 0.5 (lower keeps its value), solo objects keep the configured alpha. Iterated over several rounds (prototype, user tried each).

## Decisions & rationale
- `common: ObjProps` holds alpha/wire/normal/curv/color/mix for all objects; `effProps(f)` merges common + per-file auto colour (unlabeled models keep their auto colour when common colour is "original") + solo alpha cap. `hiddenSet`/`soloSet` persist across deselect; solo counts only selected files; hide uses `mesh.visible` (wire/normal are children).
- Info, jaw, convert-to-.lobj2 and curvature legend are per-file by nature, so they live in the shared block for an "active" object; clicking a filename in the list sets it (user's request to centralise; the active-object design is mine). Clicking also flashes a `Box3Helper` AABB for 2 s, removed when another is activated.
- wire + normal on one row. Custom colour: rainbow conic-gradient circle swatch opening `<input type=color>`; stored as `#rrggbb`, `resolveCssColor` passes `#` values straight to THREE.Color.
- `buildCard` split into `buildSettings` + `buildRow` + `buildDetail`.
- The first pass left the curvature legend per row; user noticed it wasn't fully moved.

## Implementation (files)
`web/src/main.ts`, `web/src/props.ts`, `web/src/i18n.ts` (hideTip, soloTip, customColor, customColorTip; zh/en/vi, vi machine-made), `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit` and `vite build` ok; the user tried the prototype in the browser between rounds. Final custom-colour circle look and cargo release build not checked by me.

## Open items
AABB only fires on filename click. Legend is only for the active object (ranges differ per model). Vietnamese strings unreviewed.
