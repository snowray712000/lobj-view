# 1.16.1: `convert-dir` batch .lobj → .lobj2 (CLI)

Date: 2026-11-26. Project: `lobj-view`. Version 1.16.1 (1.15.1 → 1.16.1).

## User request
Add a feature converting every `.lobj` in a directory (filename clearly saying upper or lower jaw) to `.lobj2`; trial on `/home/shared/訓練原始資料/sets_141`.

## Decisions & rationale
- CLI subcommand `lobj-view convert-dir <dir> [--dry-run] [--delete-old]`, recursive; jaw taken only from the filename (`Jaw::guess_from_path`), ambiguous names are skipped and listed, never guessed.
- Default keeps the old `.lobj` (additive, non-destructive); existing `.lobj2` is never overwritten.
- Single-file logic extracted from `/api/lobj-convert` into `lobj_api::convert_real` (write, copy times, reload-and-compare, optional delete) so CLI and web share it.

## Implementation (files)
`src/convert_dir.rs` (new), `src/lobj_api.rs`, `src/main.rs`, `Cargo.toml`/`Cargo.lock`, `README.md`, `doc/Versions.md`.

## Verification
sets_141: 434 `.lobj` → 434 `.lobj2`, 0 skipped, 0 failed (~39 s, debug build); mtime preserved. `cargo test` not run.
