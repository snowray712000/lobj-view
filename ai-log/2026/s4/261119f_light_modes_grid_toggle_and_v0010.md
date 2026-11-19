# 0.0.10: light modes (fixed / headlight / three-light) and grid checkbox

Date: 2026-11-19. Project: `lobj-view`. Version 0.0.10 (Cargo.toml bumped from 0.0.9).

## Context / User request
The user (Traditional Chinese) asked, in sequence:
1. The default light is good, but when orbiting to inspect detail the fixed light may leave that area dark. Add a way to switch: "fixed" vs "follow the eye" (like a headlamp); asked my opinion on three lights (eye, left, right). Put the control in the top-left UI block, on a **second row**, with the theme select also on that row.
2. Add a checkbox for the world Grid, default **off**.
3. Finally: write this log, wrap up, commit.

## Decisions & rationale
- Agreed three lights are a good idea → three modes in a `#light` select: `fixed` (original sky + sun, unchanged default), `head` (one DirectionalLight following the eye), `three` (head + left + right).
- Follow lights are DirectionalLights re-placed every frame in `updateLights()` from the camera basis (`matrixWorld.extractBasis`): position = `controls.target + right*x*d + up*y*d + back*z*d`, `d = max(|camera - target|, 1)`; head (0, 0.15, 1), left (-1, 0.3, 0.6), right (1, 0.3, 0.6); each light's `target` is `controls.target`. Lights and their targets are added to the scene once.
- Mode switch: `sun.visible` only in fixed; HemisphereLight intensity 1.5 (fixed) / 0.9 (follow) so the headlight shows shading; head intensity 2.2 (head) / 1.6 (three), left/right 1.0. These values are initial guesses, not tuned in a browser.
- Grid: `gridVisible` (default false) applied in `buildGrid()` (`grid.visible`); checkbox `#grid-toggle` rebuilds the grid. Axes are unaffected. Choice persisted in `localStorage` (`lobj-view-grid`, `lobj-view-light`), try/catch.
- UI: `#status` now has row 1 (title/version/server/renderer) and `#status2` (光源 · 格線 · 主題).

## Pitfalls
- `if (grid) grid.visible = ...` at top level fails `tsc` (TS narrows the module-level `let grid` to `never` since it is assigned only inside a function) → call `buildGrid()` instead.

## Implementation (files)
`web/index.html`, `web/src/main.ts`, `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `doc/Versions.md` (0.0.10), `README.md` (version line), rebuilt `web/dist`.

## Verification
- `npm run build` (tsc + vite) and `cargo build` passed.
- NOT verified in a real browser: actual lighting look/intensities, checkbox behaviour, persistence.

## Open items
Tune light intensities after the user looks at real models.
