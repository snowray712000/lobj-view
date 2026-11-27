# 1.20.3: rename folder / remove one folder level

Date: 2026-11-26. Project: `lobj-view`. Version 1.20.3. Follows [261126o](261126o_zip_extract_v1202.md).

## User request
For a selected folder: "重新命名", and "拿掉這層" (move contents up to the parent, then delete the emptied folder), replacing a manual move-then-delete.

## Decisions & rationale
- Flatten pre-checks name clashes (including a child named like the folder itself) and changes nothing on conflict; ends with `remove_dir` so non-empty folders are never deleted.
- Neither op may touch the root dir or its ancestors (would invalidate the server root).
- Same guard as other write ops (`guard_with`: local or `--admin`). Button label "拿掉這層" chosen over "檔案移出" since it names the result.
- Not done: rollback if a move fails midway.

## Verification
curl against a debug server: conflict 409 untouched, flatten, rename, existing 409, bad name 400, root 400. The user then tried the buttons in the browser and confirmed they work.
