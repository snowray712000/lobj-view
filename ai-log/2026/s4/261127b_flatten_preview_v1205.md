# 1.20.5: flatten preview dialog

Date: 2026-11-27. Project: `lobj-view`. Version 1.20.5. Follows [261127a](261127a_flatten_level_name_mask_v1204.md).

## User request
"拿掉這層" should first show a preview list, then OK / cancel; non-conforming items should show their paths so the user can inspect them.

## Decisions & rationale
- Server-side `dry_run` reuses the exact planning code, so preview and execution cannot disagree; execution still re-checks (state may change between preview and OK).
- Clashes shown as `source -> existing path`; OK is disabled while any clash exists. Lists capped at 300 entries.

## Verification
curl: dry run lists targets and the clash and changes nothing; real run returns 409. The user then tried the dialog in the browser and confirmed it works.
