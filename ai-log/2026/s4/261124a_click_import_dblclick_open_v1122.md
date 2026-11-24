# 1.12.2: File list click = add, double-click = open only

Date: 2026-11-24. Project: `lobj-view`. Version 1.12.1 → 1.12.2.

## Context / User request
In the left file list a plain click cleared the current selection and loaded the clicked file; only Ctrl/Shift kept existing models. User wanted: plain click = import (add, like old Ctrl-click); Shift-click and Ctrl-click unchanged; double-click = open only that file (the old plain click). Asked me to confirm it was sensible first, then proceed.

## Decisions & rationale
- Plain click now toggles (add/remove) like Ctrl-click; the old "select only this" branch was removed from `clickFile`.
- A double-click fires click, click, dblclick. The second click (`e.detail > 1`) is ignored so it does not toggle the file back off; `dblclick` calls new `openOnly(full)` (clear selection, select this one, set `lastClicked`).
- Known side effect: the first click of a double-click on an unselected file starts a load before dblclick clears the others, so old and new models may briefly show together.
- Grey hint under the file list (`selectHint`) extended with the double-click description in zh/en/vi (vi machine-made, unreviewed).

## Implementation (files)
`web/src/main.ts` (`clickFile`, `openOnly`, `dblclick` listener on file `li`), `web/src/i18n.ts`, `web/index.html`, `Cargo.toml`/`Cargo.lock` (1.12.2), `doc/Versions.md`, `README.md`.

## Verification
`tsc --noEmit` ok. First report said "no change in behaviour" because `web/dist` is embedded by rust-embed and had not been rebuilt: after any `web/src` edit run `npm run build` in `web/` and then `cargo build --release`. After rebuilding, the user tested and confirmed it works.

## Open items
Vietnamese hint unreviewed. Brief double-click flicker described above is accepted for now.
