# src/lib/remapDoc.ts

TS mirror of Rust `edit::remap_doc` (`docs/api/src-tauri/src/edit/remap_doc.md`).

## remapDoc

```ts
export function remapDoc(doc: EditDoc, map: TimeMap): EditDoc
```

A copy of `doc` with `zooms`, `layout` and `effects` mapped `start_ms`/`end_ms` through `map.outOf` (a region that collapses inside a cut is dropped), `camera_moves` mapped `t_ms`, every duration and every id untouched, and `trim`, `cuts`, `speed` cleared because the map has consumed them. The Rust side also rewrites `clip_ms`, which the TS `EditDoc` does not carry (a pre-existing mirror gap, left as is). Recomputed once per doc change by `useTimeMap`, never per tick; the timeline keeps the original doc (clip time), the stage gets this one (output time).
