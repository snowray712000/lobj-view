# 1.4.4–1.4.6: per-object "curv" checkbox (vertex curvature colouring) + clip slider + smoothing

Date: 2026-11-19. Project: `lobj-view`. Version 1.4.4 (Cargo.toml 1.4.3 → 1.4.4). Follows [261119k](261119k_normal_checkbox_v143.md).

## Context / User request
Implement curvature as 1.4.4. First check that `D:\coding\透明牙套\AppLabelTooth\curvature` (Python) is correct, then add it to the front end as a checkbox in the object card like wire / normal, and choose an intuitive presentation.

## Review of the Python reference
- `compute_vertex_curvature_normal_variation_edge.py`: κ_edge = |Δn·e| / |e|², per-vertex max / min over incident edges. This is a correct discrete estimate of the normal curvature along the edge direction (e·S(e)/|e|²), given unit normals (Open3D `vertex_normals` are unit). Verified: on a sphere of radius R it gives exactly 1/R.
- Caveats (not bugs): `abs` drops convex/concave sign; min |κ| is not the min principal curvature (saddles give ~0); `normalize_curvature` is a no-op for display because `color_curvature` min-max normalises again; `clip_percentile`'s type hint is odd but harmless.

## Decisions & rationale
- Computed in the front end (`web/src/curvature.ts`, pure functions) on the same normals the viewer shows (file normals or `computeVertexNormals`, re-normalised). No edge dedup: max/min unaffected; signed mean differs only on boundary vertices.
- Added a signed mode (mean of signed κ_edge; outward normals → convex positive) as the default because red = convex / blue = concave is the most readable for teeth (gingival margin and grooves turn blue). max / min kept for parity with Python, viridis like `color_curvature`.
- No std scaling (Python demo divides by std of x): keep physical units so the legend shows κ in 1/mm and a radius tooltip.
- Clipping changed after measuring the real scan `lower_00OMSZGW_.lobj2` (edge ~0.175 mm): |signed| p50 0.36, p90 1.7, p99 6.1; max p99 19. Python's 0.5–99.5% left nearly the whole model one colour, so signed uses ±p90 of |κ| and max/min use 1–95%.
- Curvature colours override color/mix while checked; unchecking restores. Computed lazily, cached in `mesh.userData.curv` / `curvColoring` (~250 ms for 180k vertices in node).

## Implementation (files)
`web/src/curvature.ts` (new), `web/src/props.ts` (`curv`, `curvMode`, `curvatureColoring`), `web/src/main.ts` (checkbox + mode select + legend, `renderLegend`), `web/src/style.css`, `Cargo.toml`, `README.md`, `doc/Versions.md`, rebuilt `web/dist`.

## Verification
Sphere unit test in node (signed = max = min = 0.2 for R=5, inward normals → −0.2). `tsc --noEmit`, `npm run build`, `cargo build` pass. Checked in the in-app browser with `lower_00OMSZGW_.lobj2`: all three modes render, legend updates, unchecking restores label colours.

## 1.4.5: clip slider (follow-up request "加一個截斷百分位的滑桿")
- One slider "截斷" (p50–p100, step 0.5) under the curv row, shown only when curv is on; value stored per mode in `ObjProps.curvClip` (defaults `DEFAULT_CLIP`: signed 90, max/min 95); double-click resets.
- signed: range ±(p-th percentile of |κ|). max/min: [min(1, 100−p), p] percentiles, so p = 99.5 reproduces Python's 0.5–99.5% and p = 100 means no clipping.
- Sorted arrays cached per mode (`Curvature.sorted`), so dragging is O(1) lookup + recolour; coloring cache keyed by `mode:clip`.
- Verified in the in-app browser: p60 gives visibly stronger contrast (±0.49), max keeps its own p95, p99.5 gives 0.049–27.3 with tooltip 0.5%–99.5%, double-click restores p90.

## 1.4.6: smoothing before computing (follow-up request "再多一個 曲率先平滑再算 選項")
- Slider "平滑" (關, ×1–×20) under the curv row; recompute on `change` (release), label updates on `input`. Curvature cached per smoothing count in `mesh.userData.curvBySmooth`; colouring cache key `smooth:mode:clip`.
- First tried smoothing positions (uniform Laplacian and Taubin λ=0.5/μ=−0.53, also 0.6/−0.63) then recomputing normals: on `lower_00OMSZGW_.lobj2` p90 barely moved (1.68 → 1.58–1.28) and p99 rose (6.1 → 6.9–7.4) because tangential drift makes slivers (small |e| → huge κ). Rejected.
- Chosen: `smoothNormals` — positions fixed, each iteration replaces a normal with the normalised mean of itself and its 1-ring. Results: ×1 p90 1.28 / p99 4.0, ×5 0.85 / 2.0, ×10 0.67 / 1.4; sphere ×10 still 0.202 (1/R = 0.2). ~0.8 s for ×5 on 180k vertices in the browser.
- Verified in the in-app browser: ×5 gives legend ±0.85 and a visibly cleaner picture (continuous blue gingival margin, red cusps/edges, speckle gone).
- Side note: a test server I started with a shell `&` (PID 21852, port 8790) could not be killed by the agent (permission denied); the user needs to close it.

## Open items
For meshes with inward normals the signed colours flip; smoothing runs on the main thread (could move to a Web Worker if ×20 on big meshes feels slow).
