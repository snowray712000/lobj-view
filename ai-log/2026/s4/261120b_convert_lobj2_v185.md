# 1.8.5: "轉 lobj2" button (legacy .lobj → .lobj2)

Date: 2026-11-20. Project: `lobj-view`. Version 1.8.5 (1.8.4 → 1.8.5). Follows [261120a](261120a_per_file_jaw_v184.md).

## Context / User request
On the right-hand object card, once a legacy `.lobj` has a jaw specified, offer a convert button writing a `.lobj2` that keeps the original file's dates, with a dialog: delete old file (default) or keep.

## Decisions & rationale
- "轉存為 .lobj" in the request was read as a typo for `.lobj2` (button name is 轉lobj2).
- Button shown when jaw is known: card choice, global select, or filename upper/lower (mirrors backend guess).
- Backend `POST /api/lobj-convert`, local-only + `X-Lobj-Requested` (writes/deletes files). Refuses zip-cache paths and existing target; verifies by reload before deleting old. Times copied via `File::set_times` (+ created on Windows); metadata read before loading since reading touches atime.

## Implementation (files)
`src/lobj_api.rs`, `web/src/main.ts`, `web/src/style.css`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
`cargo build` and `npm run build` pass. End-to-end API test against a real `.lobj` (case-test copy, dates set to 2020-03-04): keep-old and delete-old both succeed, creation/modified times preserved on the new file; existing `.lobj2` → 409; missing `X-Lobj-Requested` → 403. Browser dialog NOT clicked through.

## Open items
Batch convert of several files; non-Windows creation time not preserved.
