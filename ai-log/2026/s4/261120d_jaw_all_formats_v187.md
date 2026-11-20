# 1.8.7: jaw verdict for .stl/.ply/.obj + two-word green/red badge

Date: 2026-11-20. Project: `lobj-view`. Version 1.8.7 (1.8.6 → 1.8.7). Follows [261120c](261120c_jaw_regex_v186.md).

## Context / User request
Apply the jaw regex verdict also to `.stl` / `.ply` / `.obj` (so they can later be converted to `.lobj2`); `.lobj2` and `.zip` need no verdict. Judgement only — conversion not yet. Badge: write both `up` and `lo`, the verdict in iOS green, "abnormal" in red ("如何?" — asked for my take).

## Decisions & rationale
- "Abnormal" was not defined by the user; I defined it as the verdict contradicting the filename text (judged upper but name contains `lower`, or judged lower but name contains `upper`) and coloured the verdict red. To confirm with the user.
- Per-file card jaw select now shown for all four formats; the convert button stays `.lobj`-only. Only legacy `.lobj` loading depends on jaw, so changing jaw reloads only `.lobj` files.
- Colours via `--systemGreen` / `--systemRed` (theme aware); inactive word dim (`--tertiaryLabel`).

## Implementation (files)
`web/src/main.ts`, `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`npm run build` passes. In the in-app browser with a temp folder: stl/obj/ply/lobj rows show `up lo` with correct green word, `lower_u_x.lobj` shows red `up`, `.lobj2` blank.

## Open items
Definition of "abnormal"; converting stl/ply/obj to `.lobj2`.
