import type { LayoutSeg } from "../../lib/edit";

/** Arrange mode's whole state: whether the stage is showing the panel frames, and which layout
 *  segment they belong to. `segId` survives an `escape` so the inspector's own "Arrange on stage"
 *  button can put the user straight back into the mode they just left. */
export interface ArrangeMode { on: boolean; segId: string | null }

/** The four things that can change it: the editor selection moved (`segId` is the LAYOUT segment
 *  it landed on, or `null` for a deselect / any other kind of selection), the inspector's button
 *  was pressed, Escape was pressed, or a panel-less doc refresh dropped the segment. */
export type ArrangeEvent =
  | { kind: "select"; segId: string | null }
  | { kind: "arrange" }
  | { kind: "escape" }
  | { kind: "gone" };

export const NO_ARRANGE: ArrangeMode = { on: false, segId: null };

/** Pure transition. Selecting a layout pill IS the entry gesture (binding UX), so a `select`
 *  carrying an id turns the mode on outright - including re-selecting the same segment after an
 *  Escape. Deselecting (or selecting a zoom/effect/keyframe instead) exits and forgets the
 *  segment; Escape exits but REMEMBERS it; `arrange` re-enters, and is inert with nothing
 *  remembered. */
export function nextArrangeMode(prev: ArrangeMode, ev: ArrangeEvent): ArrangeMode {
  switch (ev.kind) {
    // Reference-stable when nothing actually changes: BOTH the `onSel` dispatch and the
    // `[selSegId]` effect fire for one pill click, and returning a fresh `{on:true,segId}` literal
    // for the second of them would defeat React's bail-out and cost an extra Editor render.
    case "select":
      if (!ev.segId) return NO_ARRANGE;
      return prev.on && prev.segId === ev.segId ? prev : { on: true, segId: ev.segId };
    case "arrange": return prev.segId ? { on: true, segId: prev.segId } : prev;
    case "escape": return prev.on ? { on: false, segId: prev.segId } : prev;
    case "gone": return NO_ARRANGE;
  }
}

/** Where the playhead has to go for arrange mode's frames to be showing something real: `null`
 *  when it is already inside the segment, else `start_ms + transition_ms` - the first instant the
 *  layout is SETTLED rather than mid-fade - clamped to stay inside `[start, end)`.
 *
 *  Not cosmetic. `layoutAt` only selects a segment while the playhead is inside its span, so from
 *  outside it the live draft has no effect on the composite at all and the brief's "dragging
 *  updates a local draft consumed by the composite loop for THIS segment" would silently not hold.
 *  Same shape as `ZoomInspector`'s seek-into-span. */
export function arrangeSeekMs(seg: LayoutSeg, timeMs: number): number | null {
  if (timeMs >= seg.start_ms && timeMs < seg.end_ms) return null;
  return Math.min(seg.start_ms + seg.transition_ms, Math.max(seg.start_ms, seg.end_ms - 1));
}
