# 1.16.4: delete .lobj whose .lobj2 already exists

Date: 2026-11-26. Project: `lobj-view`. Version 1.16.4. Follows [261126h](261126h_to_lobj2_vite_v1163.md).

## User request
Already-converted folders could not have their `.lobj` removed, since delete only happened as part of a conversion. Add a button deleting `.lobj` when the `.lobj2` exists.

## Decisions & rationale
- New `POST /api/lobj-delete-old`; before deleting, both files are loaded and vertex/triangle counts must match (labels not compared: the old file's jaw may be ambiguous). 409 if no `.lobj2`.
- Page button acts on every row with an existing `.lobj2`, after a confirm; per-row result shown.

## Verification
API on temp copies: delete OK, missing `.lobj2` → 409, no header → 403. Count-mismatch branch and browser UI not exercised.
