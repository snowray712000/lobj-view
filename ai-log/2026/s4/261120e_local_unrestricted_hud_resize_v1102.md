# 1.10.1 / 1.10.2: localhost browses outside --dir; resizable left HUD

Date: 2026-11-20. Project: `lobj-view`. Versions 1.9.1 → 1.10.1 → 1.10.2 (1.10.x = development line per Versions.md rule).

## Context / User request
1. 1.10.1: when the server listens on localhost (not 0.0.0.0), do not confine browsing to `--dir`.
2. 1.10.2: the left UI is too wide; make its width draggable, and let components wrap downward when narrow.

## Decisions & rationale
- 1.10.1: unrestricted = `Shared::local_only()` (host is loopback); 0.0.0.0 keeps confinement. Paths outside root are shown to the frontend as absolute `/`-separated (`D:/x`), `\?\` prefix stripped. Listing gets an `up` field so `..` works above `--dir` (frontend can't compute it lexically any more). Convert also unrestricted (it is already local-only). zip contents stay confined to their cache; `Resolved.in_zip` replaces `base != root`.
- Windows trap avoided: `browse("D:")` would mean the drive's cwd, so drive roots are always `D:/`.
- 1.10.2: `#hud` width = CSS var `--hud-w` (default 34em), right-edge handle with pointer events, saved to localStorage, double-click resets. `#path`/`#jaw-re` became flex-wrap; status items wrap naturally.

## Implementation (files)
`src/lobj_api.rs` (`resolve_ex`, `list_dir_ex`, `virt_of`, `parent_virt`, test `unrestricted_local`), `web/src/{main.ts,lobj.ts,style.css}`, `web/index.html`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`cargo test` 13 passed; `tsc` + `npm run build` ok; user tested both versions manually and confirmed OK. 1.10.1 committed as bebff7e.

## Gotcha
A blind "replace first `version = "1.10.1"`" in Cargo.lock hit `httparse` and broke the build; fixed by restoring httparse and bumping only `lobj-view`.

## Open items
No drive list (type `C:/` manually); Unix absolute paths untested.
