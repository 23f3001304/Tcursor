/** The click ripple's pure half: the record, the cap, and the two questions the overlay asks of
 *  whatever the pointer landed on ("does this surface want a ripple at all?" and "which tint?").
 *  Kept out of `InterfaceEffects.tsx` so both can be pinned by a unit test instead of by clicking
 *  around the editor - the tint rule in particular is a selector list that is easy to break and
 *  impossible to notice breaking. */

/** The bloom: 8px to 56px, faded out over 320ms. `RIPPLE_FROM / RIPPLE_TO` is the initial `scale`
 *  the overlay animates to 1 - the element is drawn at its FINAL size and only ever scaled, so the
 *  ripple costs one composited transform and never a layout. */
export const RIPPLE_FROM = 8;
export const RIPPLE_TO = 56;
export const RIPPLE_MS = 320;
/** At most this many live at once. A cap, not a rate limit: a fast clicker still gets a ripple per
 *  click, but the seventh evicts the oldest (which exits through `AnimatePresence` rather than
 *  popping), so the overlay can never accumulate nodes under a stuck pointer or a drag. */
export const RIPPLE_CAP = 6;

/** Which of the two tints a ripple carries. `rim` is the quiet neutral hairline - the default for
 *  every surface in the editor. `accent` is the app's red, reserved for the controls that are
 *  already saying something: the transport's hero, Export, and anything currently engaged. */
export type RippleTone = "rim" | "accent";

/** One live ripple. `x`/`y` are CLIENT coordinates - the overlay is `position: fixed; inset: 0`,
 *  so they drop straight into `left`/`top` with no measuring. */
export interface Ripple { id: number; x: number; y: number; tone: RippleTone }

/** Surfaces that own their pointer outright and must stay un-rippled: the stage frame (click to
 *  zoom, click to aim, drag the reticle / the PiP / an arrange panel) and the timeline's track
 *  stack (scrub, drag a pill, drag a handle). Marked in the markup rather than listed here by
 *  class, so a new surface opts out where it is written instead of by editing this file. */
export const FX_OFF_ATTR = 'data-ui-fx="off"';
const FX_OFF_SELECTOR = `[${FX_OFF_ATTR}]`;

/** The accent tint's targets: the play button, the Export button, and any engaged control - `.on`
 *  is the editor's own "this toggle is currently on" class (trim pills, tool toggles, view mode,
 *  lane labels) and `aria-current` is the rail's active slot. */
const ACCENT_SELECTOR = ".e-play, .e-export, .on, [aria-current]";

/** Is this element inside a surface that has opted out of ripples? Walks ancestors, so marking a
 *  wrapper covers everything drawn inside it. A null target (a pointerdown on the document itself)
 *  is NOT suppressed - that is the editor's own ground, which ripples like anything else. */
export function suppressesRipple(el: Element | null): boolean {
  return el !== null && el.closest(FX_OFF_SELECTOR) !== null;
}

/** The tint for a ripple landing on `el`: `accent` if it or any ancestor is one of the controls
 *  above, else the neutral ring. Ancestors, not the element itself, because the pointer usually lands
 *  on an `<svg>` glyph or a `<span>` label INSIDE the button that carries the class. */
export function rippleTone(el: Element | null): RippleTone {
  return el !== null && el.closest(ACCENT_SELECTOR) !== null ? "accent" : "rim";
}

/** Append a ripple, evicting from the front once past `RIPPLE_CAP`. Returns a new array (the
 *  caller is `setState`), oldest first. */
export function pushRipple(list: readonly Ripple[], r: Ripple): Ripple[] {
  const next = [...list, r];
  return next.length > RIPPLE_CAP ? next.slice(next.length - RIPPLE_CAP) : next;
}

/** Remove one ripple by id. Returns the SAME array reference when the id is not present, so the
 *  `setState` React bails out of it - an already-evicted ripple whose exit animation then completes
 *  must not cost a re-render of the overlay. */
export function dropRipple(list: Ripple[], id: number): Ripple[] {
  return list.some((r) => r.id === id) ? list.filter((r) => r.id !== id) : list;
}
