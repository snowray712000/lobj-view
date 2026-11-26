# 1.16.3: `/to-lobj2` rebuilt as a second Vite entry

Date: 2026-11-26. Project: `lobj-view`. Version 1.16.3. Follows [261126g](261126g_to_lobj2_page_v1162.md).

## User request
Rewrite the standalone page with Vite so it can share components (folder picker etc.) with the viewer.

## Decisions & rationale
- `vite.config.ts` multi-page input (`index.html`, `to-lobj2.html`); shared code splits into a common chunk, the new page does not pull in three.js.
- `picker.ts` left untouched: `initPicker(deps)` is wired with a page-local `selected` set (= files ticked for conversion); the picker HTML block is duplicated in `to-lobj2.html` (ids are what the module binds to). A future cleanup could extract that block.
- i18n: `l2*` keys added to zh/en/vi. Page theme read from the viewer's `lobj-view-theme`. `style.css` sets `body{overflow:hidden}`, overridden in `src/to-lobj2.css`.
- Removed the static `web/public/to-lobj2.html`; backend route unchanged.

## Verification
`npm run build` (tsc + vite) passes; served page and its assets return 200 from the Rust server. NOT verified in a browser (neither the new page nor the viewer under the new bundle layout).
