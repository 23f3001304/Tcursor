import type { PanelRectDto } from "../../lib/ipc";
import type { ResolvedPanels } from "./layoutTrack";
import { VISIBLE_ALPHA } from "../stage/arrange/arrangeMath";

/** The schematic's own coordinate box - a 24x14 unit rect (roughly the output frame, scaled way
 *  down). `LayoutThumb.tsx`'s `<svg viewBox>` uses these SAME numbers, so a bigger rendered size
 *  (`LayoutInspector`'s 48x28 header thumbnail) is a pure SVG/CSS scale-up of the identical
 *  geometry, not a second computation. */
export const THUMB_W = 24, THUMB_H = 14;

/** One panel's box inside the schematic, or `null` when it isn't shown. */
export interface ThumbBox { x: number; y: number; w: number; h: number }
export interface ArrThumb { screen: ThumbBox | null; cam: ThumbBox | null }

function box(p: PanelRectDto, w: number, h: number): ThumbBox | null {
  return p.alpha > VISIBLE_ALPHA ? { x: p.rect[0] * w, y: p.rect[1] * h, w: p.rect[2] * w, h: p.rect[3] * h } : null;
}

/** A schematic of a segment's RESOLVED panels (`resolvedPanelsFor`, T34 L2's per-segment channel):
 *  `PanelRectDto.rect` is already a 0..1 fraction of the output frame, so "normalizing into the
 *  thumb box" is just scaling those same fractions by `w`/`h` - zero pose math, the same rule
 *  `layoutTrack.ts` follows (this only re-scales an already-resolved rect, never derives one). A
 *  panel resolved at/below `VISIBLE_ALPHA` (hidden - e.g. `camera_only`'s screen, or a segment with
 *  one panel switched off) draws as `null`, not a zero-size box.
 *
 *  `panels === null` - presets haven't loaded yet, or there's no segment to show - returns both
 *  boxes `null` rather than guessing a placeholder; the caller (`LayoutThumb.tsx`) falls back to a
 *  plain icon in that case. */
export function arrThumb(panels: ResolvedPanels | null, w = THUMB_W, h = THUMB_H): ArrThumb {
  if (!panels) return { screen: null, cam: null };
  return { screen: box(panels.screen, w, h), cam: box(panels.cam, w, h) };
}
