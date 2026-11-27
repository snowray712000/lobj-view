# 1.20.6–1.20.12: organize page, file-name and folder-name rules

Date: 2026-11-27. Project: `lobj-view`. Versions 1.20.6 to 1.20.12. Follows [261127b](261127b_flatten_preview_v1205.md).

## User request
- Privacy masking only covered folder names; patient names also sit in file names (e.g. BCom `…_<id>A_<name>.lobj` under `/home/shared/訓練原始資料`).
- Zip / unzip / sync / delete zip / rename / flatten / mask are housekeeping, not viewing: move them to a separate page, add a page switcher to every page.
- File-name handling by regex, two modes: replace (empty = delete) and keep (first match only); savable named rules; preview; reusable one folder up with a deeper level.
- Case-insensitive regex; colored preview (Apple colors); the same rule mechanism for folder names.

## Decisions & rationale
- New page `/organize` (整理) holds all housekeeping; viewer keeps only viewing. Top toolbar buttons so a long file list cannot push them off screen; list is last.
- Rule applies to the stem only, extension always preserved, because jaw detection depends on `upper` / `lower` in file names.
- One rule editor (`mountRuleEditor`) shared by the mask dialog and the standalone dialog; one backend planner (`mask_plan`) shared by preview and execution, so they cannot disagree. Conflicts return 409 and nothing is renamed; renames go through temporary names.
- Standalone `POST /api/file-rule` works on the folder being viewed and remembers the last rule, so going up a folder and raising the level reuses it. Depth: 0 = all, N = exactly level N (1 = direct children).
- Folder rule (`target: "dirs"`): whole name is the stem (dots are not extensions), deepest first, folders containing the server root are skipped.
- `regex` crate has no lookaround; documented in the UI hint. Case-insensitive by default, `(?-i)` to turn it off.

## Verification
curl: mask/unmask round trip, replace / keep previews, bad regex → 400, depth filtering, `?all=1`, case-insensitivity, folder-name mode. `cargo check` and `npm run build` pass. Not tested in a browser: organize page dialogs, sync / zip panels, preview colors, `#folder-rule`. `cargo test lobj_api::zip_as_folder` fails, but it also fails without these changes.

## Open
- Depth is 1-based in the UI; the user spoke of "第零層" for the current folder, so 0-based numbering may be wanted.
- `no_user_info.py` and the viewer's front-end messages are not synced with the file rules (user said not needed for now).
- Masking and the folder rule are separate operations; they could be chained if wanted.
