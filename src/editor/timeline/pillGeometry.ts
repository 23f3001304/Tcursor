/** How far a region pill's edge must stay short of the TRACK's own edge, in percent of the track
 *  width, whenever the pill's raw start/end lands exactly on it (0 or the clip's duration) - so
 *  the pill never renders flush against the bare track edge OR a `TrimOverlay` in/out handle
 *  (`.e-trimhandle`, editor.css), which sits at that SAME x position whenever the clip is
 *  untrimmed (its default state).
 *
 *  Originally right-edge-only (Task 11, ux audit #26): a region running to the clip's own
 *  duration painted edge-to-edge with the trim-out handle's glowing line, reading as overflow
 *  rather than "reaches the end". Extended to the LEFT edge too (gate-feedback item 1,
 *  user-reported 2026-09-02): a full-span pill (`start_ms=0`) got NO left inset at all, so its
 *  own left resize handle (`.e-zh`, 11px - the pill's first flex child, editor.css) sat exactly
 *  where the trim-IN handle is centered (`left:0%`, `margin-left:-4.5px`, so it covers roughly
 *  -4.5..+4.5px) - the higher-z-index trim handle ate the first ~4.5px of the resize handle's hit
 *  zone, the part closest to the edge a user would actually reach for.
 *
 *  This inset is the VISUAL half of the item-1 fix, keeping the overlap small at ordinary track
 *  widths; `editor.css` also lifts a pill's own z-index above the trim handle's (6) while it's
 *  hovered/selected/dragging (fix round 1, controller ruling 2026-09-02 - NOT statically by
 *  whether the span touches an edge, which used to paint a pill over `.e-trimdim`'s dimming
 *  stripe whenever that same edge was also independently trimmed) so its resize handles stay
 *  grabbable even where a narrow track still leaves the two touching. */
export const PILL_EDGE_INSET_PCT = 0.6;

/** Below this width (percent of the track), a region reads as an unclickable sliver rather than
 *  a draggable pill - the floor every zoom/fx/layout region has always used. */
export const MIN_PILL_PCT = 2.5;

/** A region pill's left offset, in percent of the track: the raw `start/dur` position, except a
 *  region starting at (or before) the clip's own beginning is nudged in by `PILL_EDGE_INSET_PCT`
 *  instead of sitting exactly on the track's left edge - see `PILL_EDGE_INSET_PCT`'s own doc
 *  comment for why. Pills that don't start at the true edge are unaffected. */
export function pillLeftPct(startMs: number, durMs: number): number {
  if (durMs <= 0) return 0;
  if (startMs <= 0) return PILL_EDGE_INSET_PCT;
  return (startMs / durMs) * 100;
}

/** A region pill's rendered width, in percent of the track: the raw `(end-start)/dur` span,
 *  floored at `MIN_PILL_PCT` (short regions must stay draggable) and capped so the pill's rendered
 *  right edge never reaches the track's true right edge - `leftPct` defaults to whatever
 *  `pillLeftPct` would itself render at, so the two stay geometrically consistent by construction;
 *  pass the exact value `RegionRows` used for `left` if it was computed separately. Pills that
 *  don't reach either edge are unaffected on that side - the cap only bites once a pill would
 *  otherwise touch 100%, and the left nudge only applies when `startMs<=0`. */
export function pillWidthPct(startMs: number, endMs: number, durMs: number, leftPct = pillLeftPct(startMs, durMs)): number {
  if (durMs <= 0) return 0;
  const rawWidthPct = ((endMs - startMs) / durMs) * 100;
  const maxWidthPct = Math.max(0, 100 - PILL_EDGE_INSET_PCT - leftPct);
  return Math.max(MIN_PILL_PCT, Math.min(rawWidthPct, maxWidthPct));
}
