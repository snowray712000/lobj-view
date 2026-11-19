# OBJ and PLY readers, released as 0.0.7

## Context / User request
User (Traditional Chinese), in three consecutive messages: "使 lobj-view 支持 .obj 檔" -> "使支援 .ply" -> "發版 0.0.7".
Same workspace as 261119c: `rust-workspace/` with shared std-only crate `lobj/` (path dependency `../lobj`) and `lobj-view/`. Both formats follow the exact pattern set by the STL reader: a module in `lobj` that returns a `LabeledObject` with all labels = -1 (unknown colour), hooked into `lobj::load` by extension, plus the viewer's file filter.

## Decisions & rationale
- Readers live in `lobj/src/obj.rs` and `lobj/src/ply.rs`, not in the viewer, so every `lobj` consumer gets them. Read-only (no writer; `LabelScheme::from_path` still rejects these extensions, so `save` returns UnknownExtension).
- `lobj::load` checks extension `stl`, then `obj`, then `ply` (case-insensitive) before the label-scheme logic; no jaw needed.
- labels = `Some(vec![-1; n_vertices])`; the backend `encode()` already fills colours from labels, so no colour code changed.
- Per-vertex colours from the file ARE kept (`obj.colors`) when every vertex has them. Note: the backend only fills colours when `obj.colors.is_none()`, so such files show their own colours instead of the unknown colour.
- Errors: `LobjError::BadObj(&'static str)` and `BadPly(&'static str)` with Display "OBJ/PLY 格式錯誤：…".

## Implementation
### OBJ (`obj.rs`, ASCII only)
- Line-based; `#` comments stripped. `v x y z [r g b]` (>=6 numbers -> colour, used only if colour count == vertex count). `f` tokens take the part before the first `/` (so `a`, `a/t`, `a//n`, `a/t/n`); positive index is 1-based, negative is relative to the vertices read so far (`n + i`); index 0 is an error. Polygons are fan-triangulated `[ids[0], ids[k], ids[k+1]]`.
- Everything else (vt, vn, g, o, usemtl, mtllib, ...) ignored. Out-of-range indices are caught by `validate()` (TriangleIndexOutOfRange).
### PLY (`ply.rs`)
- Header found by searching bytes for `end_header`, then one optional `\r` and one `\n`. Parses `format` (ascii / binary_little_endian / binary_big_endian), `element name count`, `property <type> name`, `property list <counttype> <itemtype> name`; comment/obj_info ignored. Type names: char/int8, uchar/uint8, short/int16, ushort/uint16, int/int32, uint/uint32, float/float32, double/float64.
- Body read generically by a `Source` that returns every value as f64 (exact for all supported types). Every element/property is consumed in header order so unknown elements are skipped correctly in binary files.
- `vertex`: needs scalar x,y,z (else BadPly); red/green/blue become colours, divided by 255 if the property type is integer, taken as-is if float. `face`: list property `vertex_indices` or `vertex_index`, fan-triangulated; other list properties are read and discarded.
- Truncated binary data -> `TooShort`.

## Pitfalls & gotchas
- ASCII PLY body is taken via `std::str::from_utf8(...).unwrap_or("")`: a non-UTF-8 ASCII body yields an empty token stream and then a "資料比 header 宣告的少" error (a misleading message; the code comment about this is a bit muddled).
- PLY colour alpha, normals, texture coords, and face colours are ignored. Binary files with a `face` list using a count type wider than needed work, but `n as usize` of a corrupt huge count would allocate (`Vec::with_capacity(n)`) — not guarded.
- OBJ materials/textures (.mtl) are not read.
- Frontend text change (`web/src/main.ts` empty-folder hint) only shows after rebuilding `web/dist`; loading itself needs no frontend change.

## Release 0.0.7
- User said "發版 0.0.7" -> bumped `lobj-view/Cargo.toml` and `Cargo.lock` to 0.0.7, README version line and format list updated, `npm run build` (web/dist rebuilt, hint text confirmed present in bundle) and `cargo build --release` OK.
- Not done: git tag / push (project has no git repo at workspace level; CI path-dependency issue from 0.0.4 still open).

## Verification
- `cargo test` in `lobj`: 31 pass (3 new obj tests: quad fan + `//` form, negative index + vertex colours, errors; 3 new ply tests: ASCII, binary LE+BE, errors). `lobj-view`: 3 pass (name filter extended to .obj/.ply).
- NOT done: opening real .obj/.ply files in the browser viewer; large-file performance; PLY with double/short types or non-vertex-first element order only reasoned, not tested; running the 0.0.7 exe.

## Open items
- Smoke test with real OBJ/PLY files; decide whether to read OBJ/PLY writers; tag/push 0.0.7.
