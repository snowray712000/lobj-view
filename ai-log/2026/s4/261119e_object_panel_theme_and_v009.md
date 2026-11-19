# 0.0.9: object panel, left-side restructure, theme, bbox-scaled grid, z-up

Date: 2026-11-19. Project: `lobj-view` (axum backend + Vite/three.js WebGPU frontend in `web/`). Version 0.0.9 (Cargo.toml bumped from 0.0.8).

## Context / User request
The user (Traditional Chinese) asked, in sequence:
1. A right-hand collapsible (default expanded) object panel with per-object properties. Ideas: `alpha` slider 0–1 default 1 (needed when several models are loaded); `color` — unsure whether to overlay on existing label colours or ignore them, and wanted a "not set" state. Colours should use Apple colour CSS variables from `D:\coding\信望愛\FHLNUI\FHLNUI\index\theme` so a theme can be added later.
2. The left side no longer needs the info text; vertex/triangle counts go in the right-hand properties.
3. Left side = two blocks: server status (one line: `lobj-view @ccu-cadcam {version} … ok … WebGPU`; click the version → near-fullscreen dialog with the versions) and the file chooser (collapsible, default open, semi-transparent so it does not hide the scene).
4. Status text must not be a bare "ok"/"WebGPU"; use Apple colour vars on the left UI (green on the dialog background was unreadable); also try to theme the 3D canvas.
5. Add a "theme" combobox as the last item of the first left block with the standard four themes; the user will pick the default later.
6. Grid and x-y-z axes should scale with the loaded objects' bbox.
7. The user's data is mostly z-up → switch the world to z-up.
8. Finally: record the session. The project is not a git repo, so "commit" was interpreted as this record-keeping task; no git commit was made.

## Decisions & rationale
- **color semantics**: default = "not set" (original label/vertex colours, button "原"). When set: `out = orig*(1-mix) + tint*mix`, `mix` default 0.6 (slider "疊加", visible only when a colour is set). The user leaned toward "overlay a bit"; mix=1 is a solid replace.
- Colours are stored as **CSS variable names** (`--systemBlue` …) and resolved at apply time through a hidden probe element's computed `color` → `THREE.Color.setStyle`. A theme change just re-applies. Palette: Red Orange Yellow Green Teal Blue Indigo Purple Pink Gray.
- `apple-color-ios.css` was copied verbatim from FHLNUI (a generated file; do not hand-edit). The larger `theme-vars.css` was NOT copied (FHL-specific); `style.css` uses the raw `--system*` / `--label` / `--separator` vars directly. Status colours mimic theme-vars: `color-mix(in srgb, var(--systemGreen) 62%, var(--label))` for contrast on light backgrounds.
- A `color` vertex attribute is always created (light grey 0.8 if the file has none) and `orig` is kept so "not set" can restore. The material always has `vertexColors: true`.
- alpha<1: `transparent=true`, `depthWrite=false` (otherwise back models vanish). No extra per-object sorting beyond three.js defaults.
- Info (vertices, triangles, labels, loading/failed) moved into each right-panel card (`.card-info`, filled by `renderInfo()`); the left `#info` is kept only for path errors.
- Left status line: `server: ok · renderer: WebGPU`. The version dialog fetches `/api/versions` (Versions.md via `include_str!`, added to Cargo `include`) and renders minimal markdown (`###` heading, `- ` items). Esc / ✕ closes it.
- Theme select: "" (auto = no `data-theme`, follows system) / light / dark / hc-light / hc-dark, persisted in `localStorage` key `lobj-view-theme` (try/catch). "auto" was added by me beyond the requested four; the default stays auto until the user decides.
- Scene background = `--systemGray6`; grid colours `--systemGray` / `--systemGray3`; rebuilt on theme change. Light colours are untouched.
- Grid/axes: `updateWorld(box)` runs in `fitCamera`: `r = max(|min/max x|, |min/max y|, zExtent/2)`, `gridSize = niceCeil(2.2 r)` (1/2/5×10ⁿ), 10 divisions, axes scale `0.15*size`; with no model → 10 / 1.5.
- **z-up**: `camera.up = (0,0,1)` set BEFORE constructing OrbitControls (it captures `up`), GridHelper `rotation.x = π/2`, HemisphereLight position (0,0,1), sun (5,-7,10), fit-camera offset `(0, -2.6r, 0.6r)`.

## Rejected / not done
- Solid-colour-only or ignoring label colours (the user wanted an unset state and leaned to overlay).
- Copying all of `theme-vars.css`; a full markdown renderer; a y-up/z-up switch (offered, not requested).

## Implementation (files)
- `web/src/props.ts` (new): `ObjProps`, `defaultProps`, `PALETTE`, `resolveCssColor`, `applyProps`.
- `web/src/main.ts`: `propsOf`/`getProps`, `renderPanel`/`buildCard`/`renderInfo`, scene theme + grid/axes, theme select, version dialog, browser collapse.
- `web/src/apple-color-ios.css` (new, copied), `web/src/style.css`, `web/index.html`.
- `src/main.rs`: `/api/version` (`CARGO_PKG_VERSION`), `/api/versions` (`include_str!("../doc/Versions.md")`); `Cargo.toml` version + `include`.
- `doc/Versions.md` 0.0.9 entry.

## Pitfalls
- `#hud` has `pointer-events:none`; its `.box` children set `auto` (same trap as in 261119a).
- `camera.up` must precede `new OrbitControls`.
- `renderPanel()` rebuilds the cards on selection changes; a load finishing mid slider-drag only updates `.card-info` via `renderInfo`.

## Verification
- `npm run build` (tsc + vite) and `cargo check` passed after the changes.
- NOT verified: anything in a real browser (panel, dialog, theme switching, how label colours look on a light theme, grid sizing, z-up orientation); no real .lobj was loaded this session.

## Open items
See `doc/TODO.md`.
