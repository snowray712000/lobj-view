# 1.6.1: treat .zip as a folder (lazy extract to cache) + cancellable busy tip

Date: 2026-11-19. Project: `lobj-view`. Version 1.6.1 (Cargo.toml 1.5.1 → 1.6.1). Follows [261119l](261119l_curvature_v144.md).

## User request
Datasets sometimes contain .zip files that are not extracted; viewing one means extracting and tidying by hand. Make a .zip look like a folder, secretly extracting somewhere without touching (rewriting) the .zip. If a huge one is clicked by mistake and takes long, allow cancelling at any time so it is not entered: show a "processing" tip with a cancel button.

## Decisions
- Extract lazily on first entry into `data_local_dir/lobj-view/zipcache/<key>/` (key = hash of real zip path + size + mtime, so a replaced zip re-extracts). The zip is only read.
- Extract only wanted entries (model extensions + inner .zip); everything else is skipped. Keeps directory structure.
- Write to `<key>.partial`, rename on success; cancel/error removes it. Startup cleanup removes leftovers and caches unused > 7 days (marker file `.lobj-view-used`).
- Virtual paths: `d/pack.zip/m/a.stl`; `resolve` returns `Resolved{real, virt, base}`; for a path not on disk it normalises lexically, finds a `.zip` ancestor, extracts, and `descend`s (nested zips loop). Confinement base is the cache dir inside a zip.
- Listing: zips appear in `dirs` and also in a new `zips` array so the UI shows 📦.
- Cancel: front end `AbortController` → connection closed → axum drops the handler → `CancelOnDrop` sets an `AtomicBool` checked per 256 KB chunk (waiters on the per-key lock poll it too). No separate cancel endpoint needed.
- UI: `withBusy` shows "處理中… name（N 秒）[取消]" only after 400 ms; a new browse aborts the previous one; cancelled model loads are deselected.
- Safety: `enclosed_name`, symlink entries skipped, 8 GB total / 50 000 file caps.

## Verification
`cargo test` (9 pass, incl. new `zip_as_folder` with nested zip, `..` handling, pre-cancelled call leaves no cache, zip bytes unchanged, and zipfs filter/traversal/cancel). Manual: 2.5 GB deflated zip via curl — 1 s timeout leaves no `.partial`; full extract took 23 s (debug build). In the in-app browser: tip appears, cancel button stops it, text "已取消（停留在原資料夾）", cache dir clean.

## Open items
Non-UTF-8 (Big5/GBK) entry names, encrypted zips, extraction progress percentage, real-world large zip with Chinese names. A stale test server from an earlier session (PID 21852, port 8790) may still be running.
