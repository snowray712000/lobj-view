# 1.8.6: jaw decided by a temporary regex list

Date: 2026-11-20. Project: `lobj-view`. Version 1.8.6 (1.8.5 → 1.8.6). Follows [261120b](261120b_convert_lobj2_v185.md).

## Context / User request
Replace the global "jaw: 自動" dropdown in the file picker with an edit box holding patterns for *upper*; everything else is *lower*. `;`-separated (filenames never contain `;`), default `upper;_u_;u\d+-;ur`, case-insensitive; the file list shows a small `up` / `lo` prefix with the current verdict for the folder. Explicitly a temporary approach.

## Decisions & rationale
- Match against the basename only (including extension), JS `RegExp(seg, 'i')`; empty segments skipped; invalid segments skipped and the input turns red (title lists them) instead of blocking everything.
- Priority: per-file card select (1.8.4) > regex. Because a verdict always exists now, the 1.8.5 convert button shows on every `.lobj` card.
- Badge only on legacy `.lobj` (other formats already carry FDI labels), blank span keeps alignment.
- Backend untouched; frontend always sends `?jaw=` for `.lobj`. The old filename guess (`upper`/`lower` substring) is gone from the UI path, so a name with only `lower` is lower purely by not matching.
- Regex not persisted (kept consistent with other options; "temporary").
- `Loaded.jaw` stores the jaw used so a regex edit reloads only files whose verdict changed.

## Implementation (files)
`web/index.html`, `web/src/main.ts`, `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`npm run build` passes. In the in-app browser against a temp folder (UPPER_a / x_u_1 / u12-b / plain .lobj + a .lobj2): badges matched the default regex (plain=lo, others=up, .lobj2 blank); editing the regex updated badges live; an invalid `(` segment turned the input red; selecting u12-b then changing the regex to `zzz` re-requested it with `?jaw=lower`. Convert dialog not re-tested.

## Open items
Persisting the regex; whether to keep the filename-substring guess as a fallback.
