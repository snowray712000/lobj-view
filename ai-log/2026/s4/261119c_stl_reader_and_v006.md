# STL (binary + ASCII) reader, released as 0.0.6

## Context / User request
User (Traditional Chinese): "讓專案可以讀 .stl，先作 binary 的，再作 ascii 的。並且 .stl 因為沒有 label，所以就是以 -1 unknown 的顏色。"
Then: "發版 0.0.6" (release 0.0.6). Workspace is `rust-workspace/` with the shared crate `lobj/` (sibling of `lobj-view/`, path dependency `../lobj`). No git repo.

## Decisions & rationale
- New module `lobj/src/stl.rs` (not in `lobj-view`) so every consumer of the `lobj` crate can read STL.
- Both binary and ASCII were implemented in one go, behind one auto-detecting `stl::read_from_bytes`.
- Detection: binary iff `len == 84 + 50*n` (n = u32 at offset 80). Many binary STLs have an 80-byte header that starts with "solid", so checking the text prefix first would misdetect them. Else, if the content (after leading whitespace) starts with `solid` -> ASCII. Else fall back to binary so the error is a meaningful TooShort/TrailingBytes.
- STL has no shared vertices (3 per facet), so vertices are merged by exact f32 bit pattern (`HashMap<[u32;3],u32>`; -0.0 normalised to 0.0). Without merging, vertex_normals would be flat and counts 3x.
- File normals and attribute bytes are ignored; normals are computed later by `LabeledObject::vertex_normals`.
- labels = `Some(vec![-1; n_vertices])` (UNCERTAIN). The viewer backend `encode()` already fills colors from labels with `Palette::Unique`, so no colour code was changed; -1 shows as the unknown colour.
- `lobj::load` special-cases extension `stl` before `LabelScheme::from_path` (which still rejects .stl, so `save` to .stl returns UnknownExtension — read-only by design, not requested).
- Added `LobjError::BadStl(&'static str)` for malformed ASCII (vertex without 3 numbers, vertex count not multiple of 3).

## Rejected alternatives
- Adding an `Stl` variant to `LabelScheme`: would force `save`/label-conversion matches to handle a scheme with no labels. Early return in `load` is simpler.
- Welding vertices with a tolerance: exact match chosen (matches how STL exporters write shared corners); tolerance not requested.

## Implementation
- `lobj/src/stl.rs`: `read_from_bytes`, `read_binary`, `read_ascii`, `read_file`, private `MeshBuilder`. ASCII parser is token-based (`split_ascii_whitespace`), only reacts to `vertex x y z` (case-insensitive), ignores facet/loop keywords; non-UTF8 bytes via `from_utf8_lossy`.
- `lobj/src/lib.rs`: `pub mod stl;`, `BadStl` + Display, stl branch in `load`.
- `lobj-view/src/lobj_api.rs`: `is_lobj_name` accepts `.stl`; error text and test updated.
- `lobj-view/web/src/main.ts`: empty-folder hint mentions .stl. Frontend needed no other change (it receives the usual .lobj byte stream).
- 0.0.6 bump: `Cargo.toml`, `Cargo.lock`, README version line + `--dir` row; `npm run build` then `cargo build --release` OK.

## Pitfalls
- Binary header beginning with "solid" (see detection above) — covered by test `binary_with_solid_header_is_still_binary`.
- `trim_ascii_start` on `&[u8]` is used (stable Rust >= 1.80).

## Verification
- `cargo test` in `lobj` (25 pass, 5 new stl tests: merge+labels, solid-header binary, ASCII, binary errors, ASCII errors) and `lobj-view` (3 pass).
- `cargo build --release` OK at 0.0.6.
- NOT done: opening a real .stl in the browser viewer; large-file performance; no tag/push (no git repo; CI path-dependency issue still open).

## Open items
- Real-world STL smoke test in the viewer.
- STL writing, if ever wanted.
