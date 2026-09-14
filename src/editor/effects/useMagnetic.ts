import { useEffect, useState, type RefObject } from "react";
import { useSpring, type MotionValue } from "motion/react";
import { useReducedMotion } from "../../lib/wave/ui/useReducedMotion";
import { interfaceEffectsOn, onInterfaceEffectsChange } from "./effectsFlag";

/** Magnetic pull: a control leans toward a pointer that comes near it, and springs back when the
 *  pointer leaves. The whole point is that it happens BEFORE the hover - the control acknowledges
 *  the approach, so arriving on it feels like the pointer was caught rather than merely landed.
 *
 *  Applied to exactly two things (the transport's Play button and the Trim In/Out pills) and
 *  exported for the next one. It must never go on the rail or inside a scrolling list: a row that
 *  leans while the list under it scrolls reads as a rendering bug, and one listener per row would
 *  turn a cheap effect into a per-frame cost proportional to the list. */

/** 300/20 - stiff enough that the lean tracks the pointer rather than trailing it, loose enough
 *  that the snap back on leave overshoots once. Softer than the transport's own `PLAY_SPRING`
 *  (500/30) on purpose: this fires while the pointer is merely nearby, so it has to read as a lean
 *  rather than as the press it is anticipating. */
export const MAGNET_SPRING = { stiffness: 300, damping: 20 } as const;
/** How far outside the control's box the field reaches, and what fraction of the pointer's offset
 *  from the centre the control travels. 28/0.25 keeps the travel under ~8px on a 32px pill.
 *  `strength: 0` is the caller's own off switch (see `useMagnetic`) - a disabled control passes it
 *  so a button that will ignore the click does not lean toward the pointer inviting one, the same
 *  rule `PlayButton` already applies to its hover and press springs. */
export const MAGNET_RADIUS = 28;
export const MAGNET_STRENGTH = 0.25;

/** The four edges of a control's box, in client coordinates - `DOMRect` satisfies it structurally,
 *  which is what lets the math below be tested without a layout. */
export interface MagneticBox { left: number; top: number; right: number; bottom: number }

/** The translation `box` should take for a pointer at `(px, py)`, in px.
 *
 *  The field is measured from the box's EDGE (0 inside it), so a wide pill and a round button both
 *  get the same 28px halo instead of the pill's corners reaching further than its middle. The pull
 *  itself is measured from the box's CENTRE, so the control leans toward where the pointer is
 *  rather than merely away from the nearest edge - a pointer approaching the right edge pulls the
 *  control right, and a pointer dead-centre pulls it nowhere at all. Outside the radius: `[0, 0]`,
 *  which is the snap-back (the spring, not this function, is what makes that a settle). */
export function magneticPull(box: MagneticBox, px: number, py: number, radius: number, strength: number): [number, number] {
  const gapX = px < box.left ? box.left - px : px > box.right ? px - box.right : 0;
  const gapY = py < box.top ? box.top - py : py > box.bottom ? py - box.bottom : 0;
  if (Math.hypot(gapX, gapY) > radius) return [0, 0];
  return [(px - (box.left + box.right) / 2) * strength, (py - (box.top + box.bottom) / 2) * strength];
}

/** Two spring `MotionValue`s to spread into a `motion` element's `style` as `x`/`y`.
 *
 *  The element they go on must not animate `x`/`y` by any other route - a `whileTap` that moved
 *  `y` would hijack the same MotionValue. Every press in the transport is scale-only
 *  (`PLAY_TAP`/`PRESS_TAP`), so all three call sites hand these straight to the button's `style`.
 *
 *  Off (`interface_effects` cleared, `prefers-reduced-motion`, or a caller passing `strength: 0`
 *  for a disabled control) the hook is a no-op: the effect returns before adding any listener and
 *  both values rest at 0, so the element renders with an identity transform and costs nothing per
 *  pointer move. */
export function useMagnetic(
  ref: RefObject<HTMLElement | null>,
  radius: number = MAGNET_RADIUS,
  strength: number = MAGNET_STRENGTH,
): { x: MotionValue<number>; y: MotionValue<number> } {
  const x = useSpring(0, MAGNET_SPRING);
  const y = useSpring(0, MAGNET_SPRING);
  const reduced = useReducedMotion();
  const [on, setOn] = useState(interfaceEffectsOn);
  useEffect(() => onInterfaceEffectsChange(setOn), []);

  useEffect(() => {
    if (!on || reduced || strength === 0) { x.set(0); y.set(0); return; }
    // rAF-coalesced: a pointermove burst records the position and schedules ONE measure per frame,
    // so the `getBoundingClientRect()` here can never turn into a layout read per input event.
    let raf = 0, px = 0, py = 0;
    const apply = () => {
      raf = 0;
      const el = ref.current; if (!el) return;
      const [dx, dy] = magneticPull(el.getBoundingClientRect(), px, py, radius, strength);
      x.set(dx); y.set(dy);
    };
    const onMove = (e: PointerEvent) => {
      px = e.clientX; py = e.clientY;
      if (!raf) raf = requestAnimationFrame(apply);
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onMove);
      if (raf) cancelAnimationFrame(raf);
      x.set(0); y.set(0);
    };
  }, [on, reduced, radius, strength, ref, x, y]);

  return { x, y };
}
