# 1.14.3: progress bars for zip folder and rsync sync

Date: 2026-11-26. Project: `lobj-view`. Version 1.14.2 → 1.14.3.

- `POST /api/zip-dir` and `/api/zip-sync` now return NDJSON (`{"progress":n[,"speed":".."]}` lines, then `{"done":..}` or `{"error":".."}`). Pre-flight failures (guard, bad host, 409 exists) are still plain HTTP errors; after streaming starts the status is already 200 so failures are the last line.
- Zip: `dir_bytes` gives the denominator, `zip_add_dir` copies in 256 KB chunks calling `ZipProgress::add` (throttled 200 ms); a failed send (client gone) aborts and removes `.partial`.
- Sync: `rsync --info=progress2` stdout split on `\r`/`\n`, `parse_progress` → percent + speed; dropping the stream drops the child (`kill_on_drop`).
- Frontend: `readStream` in `main.ts`; `#op-progress` (zip) and `#sync-bar` (sync dialog). No new i18n keys.
- Verified: 300 MB random file through the real server with a temp sshd (progress 1..100 for zip, 0→100 for rsync on localhost, 409, wrong password → error line). Not verified in a browser.
