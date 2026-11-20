# 1.10.5: Collapsible labels in object cards

Date: 2026-11-20. Project: `lobj-view`. Version 1.10.4 → 1.10.5.

## Context / User request
The right-hand object panel was too crowded: each card printed the full `label:count` list. Make it collapsible, collapsed by default, with summary text showing the number of labels (e.g. "5 種label"). First implemented zh-only; after the user tested OK, translated to English and Vietnamese.

## Decisions & rationale
- `renderInfo` builds a `<details class="card-labels">`; summary = `t().labelKinds(n)`, body = the `label:count` list. Open state kept in a module-level `labelsOpen` Set (keyed by path) because `renderInfo` rebuilds the card info on every load / language switch and would otherwise re-collapse.
- `n` = number of distinct labels from `labelCounts`, so label `-1` counts as one kind. Cards with no label data show no details row.
- Strings: zh `N 種label`, en `N label(s)` (singular for 1), vi `N nhãn`. Vietnamese is machine-made like the rest of `vi`.
- Version bumped to 1.10.5 following the one-feature-one-version habit.

## Implementation (files)
`web/src/main.ts` (`renderInfo`, `info.labelKinds`, `labelsOpen`), `web/src/style.css` (`.card-labels`), `web/src/i18n.ts` (`labelKinds` in zh/en/vi), `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit`, `vite build`, `cargo build` ok. User tested the zh version manually and confirmed OK; the en/vi strings were not re-tested in a browser.

## Open items
Native review of Vietnamese still open.
