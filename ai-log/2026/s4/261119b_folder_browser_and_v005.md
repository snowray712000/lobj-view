# Folder browser for the file picker, released as 0.0.5

## Context / User request
The frontend picked files from a flat `<select>` listing only the top level of `--dir`.
User (Chinese, paraphrased): optimize the file-selection tool: (1) a `..` entry to go up a level;
(2) clickable folders to descend; (3) pasting a path (either `/` or `\`) must work, and it must
never go below (outside) the `--dir` path. Then: "發版 0.0.5" (release 0.0.5).

## Decisions & rationale
- All confinement lives in the backend (`resolve()` in `src/lobj_api.rs`): the frontend is not trusted.
  Input is trimmed, surrounding quotes stripped, `\` -> `/`. Candidates: the input itself if absolute
  (pasted absolute path under `--dir`), then `root.join(input without leading '/')`. First candidate that
  `canonicalize`s AND `starts_with(canonical root)` wins; existing-but-outside => 403, nothing => 404.
  canonicalize handles `..` and symlinks. Root is canonicalized once in `router()` (Windows gives `\?\` prefix on both sides, so comparison is consistent).
- Pasted path pointing at a file: list its parent dir and return `selected`; frontend then opens it.
- Symlink entries that resolve outside root are hidden from listings.
- Rel paths returned to frontend always use `/`; root is `""`.

## API
- `GET /api/lobj?path=<p>` -> `{path, parent, dirs:[name], files:[{name,size}], selected: name|null}` (dirs/files sorted case-insensitively).
- `GET /api/lobj/{*path}[?jaw=]` (was `{name}`; now wildcard so subfolders work; frontend encodes each segment separately). Same `resolve()` is used, so no URL bypass.
- Replaced `json_string`-built JSON by hand (no serde dependency), as before.

## Frontend
`web/index.html`: `<input id=path>` + `<ul id=files>` replace the select. `main.ts` `browse(path)` renders `📁 ..`, dirs, files;
Enter in the input calls `browse(value)`. `style.css`: `#hud` has `pointer-events:none`, so input/ul need `pointer-events:auto` (same pitfall as 261119a).

## Pitfalls
- Writing the Rust edit via a bash heredoc python script failed on quoting; the file was rewritten directly instead.
- `browse()` re-calls itself once when `selected` is set, to re-render with the selected row highlighted (slightly wasteful, harmless).

## Verification
- `cargo test`: 3 pass, including `resolve_confined` (`..`, `sub/../../secret.lobj`, backslash, absolute inside/outside root, file->parent+selected).
- `npm run build` (tsc + vite) OK; `cargo build --release` after the 0.0.5 bump.
- NOT tested: manual browser interaction, long listings, Linux/macOS path behavior, Windows drive-letter paths on other drives.

## Release 0.0.5
Bumped `Cargo.toml` / `Cargo.lock` to 0.0.5, README updated. No git repo, so no tag/push was done.

## Open items
- Tag/push still pending (project not under git; CI path-dependency issue on `../lobj` remains).
