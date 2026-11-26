# 1.18.3: /stat download button

Date: 2026-11-26. Project: `lobj-view`. Version 1.18.3. Follows [261126l](261126l_stat_view_load_v1182.md).

## User request
A button that downloads the .csv; default file name `stat-{dataset_name}.csv`.

## Decisions & rationale
- Client-side Blob + `<a download>` from the displayed rows via the shared `buildCsv()` (UTF-8 + BOM); no server round trip, so it also works when browsing remotely. Works for both loaded and freshly computed data. Illegal filename characters become `_`.

## Verification
Build passes. Actual browser download NOT clicked through.
