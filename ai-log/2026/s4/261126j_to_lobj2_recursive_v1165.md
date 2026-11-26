# 1.16.5: "include subfolders" in `/to-lobj2`

Date: 2026-11-26. Project: `lobj-view`. Version 1.16.5. Follows [261126i](261126i_delete_old_lobj_v1164.md).

## User request
A folder like `sets_141` has no files of its own to tick; add a checkbox that lists `.lobj` of subfolders too.

## Decisions & rationale
- Client-side recursion over the existing `/api/lobj` (8 concurrent requests, zips not entered); no backend change. Rows show the relative sub-path; `exists` is computed per directory. A generation counter drops stale scans.
- Convert / delete-old operate on the whole listed set.

## Verification
`npm run build` passes, server serves the page. Not exercised in a browser (Claude in Chrome was not connected).
