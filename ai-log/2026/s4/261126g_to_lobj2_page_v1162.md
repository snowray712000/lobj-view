# 1.16.2: standalone `/to-lobj2` batch-convert page

Date: 2026-11-26. Project: `lobj-view`. Version 1.16.2. Follows [261126f](261126f_convert_dir_cli_v1161.md).

## User request
New UI, not integrated into the main viewer, e.g. at `/to-lobj2`.

## Decisions & rationale
- Static `web/public/to-lobj2.html` (vanilla JS, copied to `web/dist` by hand so no full Vite rebuild); `static_handler` maps `/to-lobj2` to it. Reuses `/api/lobj` (listing) and `/api/lobj-convert` (one file per request, sequential, progress bar).
- Jaw from filename; ambiguous rows get a manual select; rows with an existing `.lobj2` are disabled. Delete-old is opt-in with a confirm.

## Verification
Page served 200 text/html; convert API tested on a temp copy. Browser UI NOT clicked through.
