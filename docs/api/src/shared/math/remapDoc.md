# src/shared/math/remapDoc.ts

TS mirror of Rust `edit::remap_doc` (`docs/api/src-tauri/src/edit/remap_doc.md`).

## remapDoc

```ts
export function remapDoc(doc: EditDoc, map: TimeMap): EditDoc
```

A copy of `doc` with `zooms`, `layout`, `effects`, `texts` and `captions` mapped `start_ms`/`end_ms` through `map.outOf` (a region that collapses inside a cut is dropped), each caption's own `words` mapped the same way (otherwise the word-by-word highlight would drift off the syllable inside a speed span), `camera_moves` mapped `t_ms`, every duration and every id untouched, and `trim`, `cuts`, `clips`, `speed` cleared because the map has consumed them (`texts` joins the moved lists, `clips` joins the cleared ones). `clip_ms` becomes `outDurMs(map)`, exactly as `remap_doc` sets `out.clip_ms = map.out_dur_ms()` (the mirror gap closed on 2026-09-15). Recomputed once per doc change by `useTimeMap`, never per tick; the timeline keeps the original doc (clip time), the stage gets this one (output time).
