# src/editor/timeline/filmstripPlan.ts

How many frame tiles the filmstrip asks for, and how tall it asks for them. `useEditorData`'s `ensureThumbs(folder, FILMSTRIP_COUNT, FILMSTRIP_HEIGHT)` is the only caller; the height must also match `.e-filmstrip`'s own `height` in `timeline.css`, and the pair must match Rust's `thumbs::FILMSTRIP_COUNT`/`FILMSTRIP_HEIGHT` (what `preprocess::rest` pre-renders with) or the editor asks for a `thumbs_<count>_<height>` cache dir the background pass never filled and pays for a second ffmpeg run.

Kept out of `Filmstrip.tsx` so the count rule is a pure, testable function instead of a magic number at a call site (and out of a file named `filmstrip.ts`, which would collide with `Filmstrip.tsx` on a case-insensitive filesystem - TypeScript rejects the pair outright).

## FILMSTRIP_HEIGHT

```ts
export const FILMSTRIP_HEIGHT = 80;
```

`.e-filmstrip`'s drawn height in CSS px. Thumbnails are generated at exactly this height, so the strip never upscales a smaller cached JPEG into a blurry tile - the reason `ensure_thumbs` grew a `height` argument at all (it was hard-coded to `scale=-2:64`, cached in `thumbs_<count>_64`, while the lane drew at 54 and now draws at 80).

## EDITOR_TRACK_W

```ts
export const EDITOR_TRACK_W = 1440 - 88 - 16;
```

The track's width in the editor's usual window: `App.tsx` opens the editor at `min(1440, availWidth - 120)` logical px, and `.e-timeline` spends 88px of that on the lane label gutter plus 16px on its right padding. A narrower window (the 880px floor `App.tsx` sets) keeps the same nine tiles and simply crops each one harder - the count is a constant on purpose, because it is half of a cache key shared with a Rust pass that has no viewport to measure.

## filmstripCount

```ts
export function filmstripCount(trackWidthPx: number, tileHeightPx: number): number
```

Tiles that fill `trackWidthPx` at `tileHeightPx` without squashing them: `round(trackWidthPx / (tileHeightPx * 16/9))`, floored at 8 and capped at 24. The floor is Rust's own (`count.clamp(8, 120)`); the cap is where a tile gets narrower than a thumbnail is tall and the strip goes back to reading as one repetitive smear. 16:9 is assumed for the tile slot - the thumbnails are drawn `object-fit: cover`, so the assumption only decides how many tiles fit before they start cropping, never whether they look right.

### Inputs

- `trackWidthPx: number` - the filmstrip's drawn width (`EDITOR_TRACK_W` at the call site).
- `tileHeightPx: number` - the lane's drawn height (`FILMSTRIP_HEIGHT` at the call site).

### Returns

The tile count, 8..24.

## FILMSTRIP_COUNT

```ts
export const FILMSTRIP_COUNT = filmstripCount(EDITOR_TRACK_W, FILMSTRIP_HEIGHT);
```

`9` - nine ~148px tiles across a 1336px track, each within a few percent of its native 16:9 shape. The old fixed 16 made every tile 83px wide: narrower than tall, cropped to a sliver of the frame, and so nearly identical to its neighbours that the strip read as wallpaper rather than as the recording (owner, 2026-09-14: "the timeline thumbnails are extremely small and repetitive").

### Used by

- `src/editor/hooks/useEditorData.ts` - the one `ensureThumbs` call.
- `src-tauri/src/export/preview/thumbs.rs` - mirrors the pair as `FILMSTRIP_COUNT`/`FILMSTRIP_HEIGHT` for `preprocess::rest`; its own unit test asserts the pair still names `thumbs_9_80`.
