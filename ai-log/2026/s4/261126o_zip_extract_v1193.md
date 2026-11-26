# 1.19.3: unzip button for a selected .zip

Date: 2026-11-26. Project: `lobj-view`. Version 1.19.3. Follows [261126n](261126n_admin_flag_v1192.md).

## User request
A selected .zip also gets an extract (解壓縮) button.

## Decisions & rationale
- Extract next to the zip. Single top-level folder (what "壓縮資料夾" produces) -> expand as is, so zip then unzip round-trips; otherwise into a folder named after the zip, to avoid littering the parent.
- Existing result folder -> 409, then confirm overwrite (merge/overwrite files, nothing deleted).
- Same guard as zip-dir (`guard_with`): local-only or `--admin`. `enclosed_name()` blocks zip-slip.

## Verification
curl against a debug server: single top, 409, overwrite, flat zip, zip-slip entry skipped, missing 404. Browser button NOT clicked through.
