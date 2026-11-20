# 1.8.4: per-file jaw for legacy .lobj

Date: 2026-11-20. Project: `lobj-view`. Version 1.8.4 (Cargo.toml 1.8.3 → 1.8.4). Follows [261119q](261119q_single_side_v183.md).

## Context / User request
Legacy `.lobj` labels are arch_index, so upper/lower must be known. There was a single global jaw select, assuming one file. With two files with irregular names (one upper, one lower) they could not be loaded together.

## Decisions & rationale
- Backend untouched: `?jaw=` is already per request. Only the frontend passed one global value.
- Per-file jaw lives in `jawOf` (Map), shown as a select on the object card only for `.lobj` (not .lobj2/stl/obj/ply). Global select kept as default; priority card > global > filename guess.
- Changing a card's jaw reloads only that file; changing the global one reloads only legacy files without own setting.
- Stale in-flight loads (jaw changed mid-fetch) are discarded by comparing `jawUsed` with `effectiveJaw(f)`.

## Implementation (files)
`web/src/main.ts`, `web/src/style.css`, `web/index.html` (tooltip), `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`npm run build` and `cargo check` pass; not exercised in a browser with two real legacy .lobj files.

## Open items
jaw choice is not persisted across sessions.
