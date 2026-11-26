# 1.18.2: /stat shows aggregate, loads existing stat.csv

Date: 2026-11-26. Project: `lobj-view`. Version 1.18.2. Follows [261126k](261126k_stat_page_v1172.md) (committed as 1.18.1).

## User request
If the folder already has stat.csv, load it directly and offer "regenerate" (same for subfolders). Show statistics results, not the CSV-like per-case table: rows per tooth, mobile friendly; a button to transpose for desktop.

## Decisions & rationale
- Interpretation of "FDI | upper | lower" columns: items = upper, lower, each FDI; columns = item | cases | %. (Per-tooth upper/lower columns would be meaningless.) Transposed = items across, rows cases / %. Preference in localStorage.
- `GET /api/stat-csv` returns the file or 404; client parses CSV (BOM, quotes). Save is enabled only for freshly computed results; loading never rewrites the file.
- Subfolder navigation via folder buttons; cases with remarks listed under the table.

## Verification
stat-csv API (404 / content with BOM), parseCsv unit-checked with node; build passes. Browser UI NOT clicked through. Not committed.
