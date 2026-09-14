/** How many frame tiles the filmstrip asks for, and how tall it asks for them. Both feed the
 *  `ensureThumbs` request in `useEditorData`; the height must also match `.e-filmstrip`'s own
 *  height in `timeline.css`, and both must match Rust's `thumbs::FILMSTRIP_*` (which
 *  `preprocess::rest` pre-renders with) or the editor asks for a `thumbs_<count>_<height>` cache
 *  dir the background pass never filled. Kept out of `Filmstrip.tsx` so the count rule is a pure,
 *  testable function rather than a magic number at a call site. */

/** The tile aspect a screen recording almost always has. The thumbnails are drawn `object-fit:
 *  cover`, so this only decides how many tiles fill the lane before they start cropping. */
const TILE_ASPECT = 16 / 9;

/** `.e-filmstrip`'s drawn height in CSS px (timeline.css). Thumbnails are generated at exactly
 *  this height, so the strip never upscales a smaller cached JPEG into a blurry tile. */
export const FILMSTRIP_HEIGHT = 80;

/** The track's width in the editor's usual window: `App.tsx` opens the editor at
 *  `min(1440, availWidth - 120)` logical px, and `.e-timeline` spends 88px of that on the lane
 *  label gutter plus 16px on its right padding. */
export const EDITOR_TRACK_W = 1440 - 88 - 16;

/** Tiles that fill `trackWidthPx` at `tileHeightPx` without squashing them: one per tile-aspect
 *  slot, floored at 8 and capped at 24 (past that a tile is narrower than a thumbnail is tall and
 *  the strip goes back to reading as one repetitive smear). Rust re-clamps to 8..120 anyway. */
export function filmstripCount(trackWidthPx: number, tileHeightPx: number): number {
  const slots = Math.round(trackWidthPx / (tileHeightPx * TILE_ASPECT));
  return Math.min(24, Math.max(8, slots));
}

/** The count the editor actually requests: nine ~148px tiles across a 1336px track, each within a
 *  few percent of its native 16:9 shape. The old fixed 16 made every tile 83px wide - narrower
 *  than tall, cropped to a sliver of the frame, and so nearly identical to its neighbours that the
 *  strip read as wallpaper instead of as the recording. */
export const FILMSTRIP_COUNT = filmstripCount(EDITOR_TRACK_W, FILMSTRIP_HEIGHT);
