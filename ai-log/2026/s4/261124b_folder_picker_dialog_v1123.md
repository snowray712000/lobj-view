# 1.12.3: Folder picker dialog ("Browse…")

Date: 2026-11-24. Project: `lobj-view`. Version 1.12.2 → 1.12.3.

## Context / User request
Someone asked for a Windows-file-dialog-like folder chooser on the left UI. After discussion the user chose: add a new button (do not replace the existing list); dialog left = common places (Documents/Downloads/Desktop/user, drives, Linux home), pinned (pin icon) and recently opened folders; right = the folder listing with `..`, folders and supported files. Interaction: click = import, double-click = clean and import, Ctrl/Shift = multi-select (same as main list); "Open" switches the main list to the dialog's folder. Follow-up: make it resizable and/or semi-transparent so the main view change is visible while clicking. User tested and confirmed OK.

## Decisions & rationale
- Non-modal floating panel (draggable header), not a blocking modal, because click-import takes effect immediately and the user must see the 3D view react.
- Dialog keeps its own current folder; the main `curDir` only changes on "Open" (`browse(dir)`).
- Selection is shared: `picker.ts` gets main.ts's `selected` Set plus callbacks via `initPicker` deps (`applySelection`, `openOnly`, `browse`, `withBusy`); shift-range uses the dialog's own file list.
- Backend `GET /api/places` (`lobj_api.rs::places_for`): `--dir` always; home/desktop/documents/downloads (crate `dirs`) and Windows drives only when `shared.local_only()`, so remote mode never leaks the home dir.
- Pinned/recent in `localStorage` (`lobj-view-pins`, `lobj-view-recent`, recent max 12, recorded in `browse()` success via `recordRecent`). Paths are relative to `--dir`, so a changed `--dir` can make old entries fail (error shown, ✕ removes).
- Resize via CSS `resize: both` + `overflow:hidden`; size saved by `ResizeObserver`; opacity slider sets `--pk-alpha` used in a `color-mix` background (text stays opaque). `box-sizing: border-box` needed or restored size grows by the border each time.

## Implementation (files)
`src/lobj_api.rs`, `web/src/picker.ts` (new), `web/src/main.ts`, `web/src/i18n.ts` (zh/en/vi; vi machine-made), `web/index.html`, `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `doc/Versions.md`, `README.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit`, `cargo check` ok. In the browser pane: places API, listing, click/Ctrl/double-click selection, pin, "Open" + recent, opacity slider, size persistence. Not tested: Shift range in dialog, remote (`--host 0.0.0.0`) mode, en/vi display, failed-path error.

## Open items
Vietnamese strings unreviewed. Pins/recent are per-browser. Low alpha (~0.5) makes text overlap background content; default 0.85.
