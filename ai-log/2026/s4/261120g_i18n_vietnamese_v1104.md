# 1.10.4: Vietnamese UI language

Date: 2026-11-20. Project: `lobj-view`. Version 1.10.3 → 1.10.4.

## Context / User request
After the user confirmed 1.10.3 (zh + English, `lang` button) works, add Vietnamese as 1.10.4. (1.10.3 committed as 754067b.)

## Decisions & rationale
- Added `const vi: Dict` in `web/src/i18n.ts`; `type Lang = 'zh' | 'en' | 'vi'`; `DICTS` and `LANGS` (`{ code: 'vi', name: 'Tiếng Việt', html: 'vi' }`). No other code change — the 1.10.3 design held: `Dict` typing guaranteed every key was present.
- Terminology: hàm trên / hàm dưới (upper / lower jaw), nhãn (label), đỉnh / tam giác (vertex / triangle), độ cong (curvature), lồi / lõm (convex / concave), pháp tuyến (normal), chiếu trực giao (orthographic), phân vị (percentile). `upper` / `lower` kept in the jaw select as `upper (trên)` / `lower (dưới)` because filenames and regex use the English words. Swatch "original" glyph `∅` as in English.
- Translation is machine-made; native-speaker review of dental terms is still open.

## Implementation (files)
`web/src/i18n.ts`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`tsc --noEmit` + `vite build` ok; `cargo build` ok. Built-in browser: lang menu lists 中文 / English / Tiếng Việt; switching to vi re-rendered status bar, file browser, object card and server stats with correct diacritics, `<html lang="vi">`, saved to localStorage; switched back to zh; no console errors. User tested manually and confirmed OK.

## Open items
Native review of Vietnamese; the path placeholder is long and gets truncated at the default HUD width (also true in English).
