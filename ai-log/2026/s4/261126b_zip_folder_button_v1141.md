# 1.14.1: "Zip folder" button

Date: 2026-11-26. Project: `lobj-view`. Version 1.13.3 → 1.14.1.

- Goal (user): zip the current folder so it can be sent to another computer. First built as a streamed browser download; user said no download — it must write a zip on disk (later to be synced by rsync over ssh).
- Backend: `POST /api/zip-dir` `{path, overwrite}` in `src/lobj_api.rs` (`zip_dir`, `zip_add_dir`). Local-only + `X-Lobj-Requested` (via `assoc::guard_with`). Writes `<parent>/<name>.zip` through `<name>.zip.partial` then rename; 409 if exists and no `overwrite`. Zips everything (no extension filter), skips symlinks and unreadable files, keeps empty dirs. Rejects root-less paths and paths inside a zip.
- Frontend: `#zip-dir` button next to "瀏覽…" (`web/index.html`), handler in `main.ts` (confirm on 409, result in `#info`), i18n `zipDir`, `zipDirTip`, `zipping`, `zipOverwrite`, `zipDone` in zh/en/vi. `web/dist` rebuilt (rust-embed).
- Verified: curl on a temp folder (zip content listed, second call 409). Not verified: clicking the button in a browser.
- Next (not done): second button to rsync a chosen zip to another machine over ssh (login via ssh config alias vs user@host still to decide).
- Not committed: unrelated line-ending-only diffs (TODO.md, instance.rs, lobj.ts, props.ts, css, older ai-log files).
