/** Live DOM anchoring for the AI director's fake pointer (`DirectorPointer.tsx`) - pure
 *  measurement functions, no state, no side effects, so they're trivially unit-testable with
 *  stub `getBoundingClientRect` rects. Every caller re-queries the DOM and re-measures on EACH
 *  step (nothing is cached across a run), so a window resize mid-run never leaves the pointer
 *  aiming at a stale rect - see `useDirector.ts`'s per-step `document.querySelector`. */

export type Lane = "zoom" | "trim-in" | "trim-out";

/** Rows a lane's y-center comes from, when it has one - only the zoom track stacks into layered
 *  rows (`Timeline.tsx`'s `.e-zoomrow`); the trim handles have no row of their own, so those
 *  lanes fall through to the track's own vertical center below. */
const ROW_SELECTOR: Partial<Record<Lane, string>> = { zoom: ".e-zoomrow" };

/** Where the fake pointer should aim to act at `ms` on `lane`, measured against the timeline
 *  track element (`.e-tlbody`) right now. `x` is a pure ms/dur fraction of the track's rect
 *  (clamped to the track's own edges); `y` is the vertical center of the lane's row when one
 *  exists, else the track's own vertical center.
 *
 *  For the zoom lane this targets the BOTTOM-most row (last in DOM order) - `Timeline.tsx` renders
 *  the highest layer first and layer 0 last, and a freshly-added zoom always lands on layer 0, so
 *  that's the row a fresh `add_zoom_full` will actually appear in. */
export function timelinePointForMs(trackEl: HTMLElement, ms: number, dur: number, lane: Lane): { x: number; y: number } {
  const rect = trackEl.getBoundingClientRect();
  const frac = dur > 0 ? Math.min(1, Math.max(0, ms / dur)) : 0;
  const x = rect.left + frac * rect.width;

  const sel = ROW_SELECTOR[lane];
  const rows = sel ? trackEl.querySelectorAll<HTMLElement>(sel) : null;
  const row = rows && rows.length ? rows[rows.length - 1] : null;
  const y = row ? rectCenter(row).y : rect.top + rect.height / 2;
  return { x, y };
}

/** The midpoint of any element's current rect. */
export function rectCenter(el: Element): { x: number; y: number } {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

/** The center of a just-applied zoom/effect pill, found by the `data-region-id` attribute
 *  `Timeline.tsx` puts on each one - used to settle the pointer onto the REAL pill a step just
 *  created, once its id is known (only after `applyEditOp` resolves). `null` when no such pill
 *  is mounted (e.g. it landed off-screen, or the id is stale). */
export function pillPoint(trackEl: HTMLElement, regionId: string): { x: number; y: number } | null {
  const pill = trackEl.querySelector(`[data-region-id="${regionId}"]`);
  return pill ? rectCenter(pill) : null;
}

/** The center of the currently-mounted "wand" anchor (`data-director-anchor="wand"` - the
 *  Transport wand button and the AiPanel run button both carry it). When both are mounted at
 *  once (the AI panel is open), document order wins: the AiPanel button sits earlier in the DOM
 *  than Transport, so the run visibly starts from whichever one the user can actually see. */
export function anchorPoint(name: string): { x: number; y: number } | null {
  const el = document.querySelector(`[data-director-anchor="${name}"]`);
  return el ? rectCenter(el) : null;
}
