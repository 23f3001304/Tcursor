# src/editor/effects/useMagnetic.ts

Magnetic pull: a control leans toward a pointer that comes near it and springs back when the pointer leaves.

**The point is that it happens before the hover.** The control acknowledges the *approach*, so arriving on it feels like the pointer was caught rather than merely landed. Hover and press already have their own language in `stage/transportMotion.ts`; this is the beat before both.

**Where it may be applied.** Two things today - the transport's Play button (`stage/Transport.tsx`) and the Trim In/Out pills (`stage/TransportTools.tsx`) - and it is exported for the next one. It must **never** go on the rail or inside a scrolling list: a row that leans while the list under it scrolls reads as a rendering bug, and one window listener per row turns a cheap effect into a per-frame cost proportional to the list length.

## MAGNET_SPRING

```ts
export const MAGNET_SPRING = { stiffness: 300, damping: 20 } as const
```

Stiff enough that the lean tracks the pointer instead of trailing it, loose enough that the snap back on leave overshoots once. Deliberately **softer** than the transport's own `PLAY_SPRING` (500/30): this fires while the pointer is merely nearby, so it has to read as a lean rather than as the press it is anticipating.

## MAGNET_RADIUS

```ts
export const MAGNET_RADIUS = 28
```

How far outside the control's box the field reaches, in px.

## MAGNET_STRENGTH

```ts
export const MAGNET_STRENGTH = 0.25
```

What fraction of the pointer's offset from the box centre the control travels. 28/0.25 keeps the travel under ~8px on a 32px pill - enough to notice, not enough to make a row of controls look loose.

**`strength: 0` is the caller's own off switch.** A disabled control passes it, so a button that will ignore the click does not lean toward the pointer inviting one - the same rule `PlayButton` already applies to its hover and press springs. At 0 the hook adds no listener at all.

## MagneticBox

```ts
export interface MagneticBox { left: number; top: number; right: number; bottom: number }
```

The four edges of a control's box in client coordinates. `DOMRect` satisfies it structurally, which is exactly what lets [magneticPull](#magneticpull) be tested without a layout.

## magneticPull

```ts
export function magneticPull(box: MagneticBox, px: number, py: number, radius: number, strength: number): [number, number]
```

The translation `box` should take for a pointer at `(px, py)`, in px. Pure; `useMagnetic.test.ts` pins it.

**The field is measured from the box's EDGE** (0 when the pointer is inside it), so a 64px-wide pill and a 38px round button both get the same 28px halo instead of the pill's corners reaching further than its middle. Because the gaps are combined with `Math.hypot`, the field is a **rounded rect**, not a bounding box: 24px past the right edge *and* 24px past the bottom is 33.9px of true distance and out of range, while 24px past one edge alone is in.

**The pull is measured from the box's CENTRE**, so the control leans toward where the pointer actually is rather than merely away from the nearest edge. A pointer approaching the right edge pulls the control right; a pointer dead-centre pulls it nowhere at all.

Outside the radius the result is `[0, 0]` - the snap-back. This function makes it a *jump*; the spring in the hook is what makes it a settle.

## useMagnetic

```ts
export function useMagnetic(
  ref: RefObject<HTMLElement | null>,
  radius?: number,   // MAGNET_RADIUS
  strength?: number, // MAGNET_STRENGTH
): { x: MotionValue<number>; y: MotionValue<number> }
```

Two spring `MotionValue`s to spread into a `motion` element's `style` as `x`/`y`.

### The `x`/`y` channel must be free

The element these go on must not animate `x` or `y` by any other route. A `whileTap` that moved `y` would call `.set()` on the same MotionValue and hijack it. Every press in the transport is scale-only (`PLAY_TAP` is `{ scale: 0.94 }`, `PRESS_TAP` is `{ scale: 0.96 }`), so the translate channel is free at all three call sites and the values go straight onto the `motion.button`'s `style` - no wrapper element anywhere. A future caller whose press moves `y` would need one.

### Cost

One `pointermove` listener on `window` per instance (three today), registered `{ passive: true }`.

It is **rAF-coalesced**: a move event only records the coordinates and schedules one measure per frame, so the `getBoundingClientRect()` can never become a layout read per input event. The rect is re-measured each frame rather than cached, because the transport bar moves whenever the window resizes or a panel opens, and a cached rect would leave the field somewhere the control no longer is.

### Off

`interface_effects` cleared (via [effectsFlag](effectsFlag.md)), `prefers-reduced-motion` set (via `lib/wave/ui/useReducedMotion`), or `strength === 0`: the effect returns before adding any listener, both values are `set(0)`, and the element renders with an identity transform. The flag is subscribed on mount, so flipping it in Preferences stops the pull on a transport that is already on screen without a reload.

Reduced motion turns the pull **off entirely** rather than shortening it: the whole effect is motion that the user did not ask for, with no information in it.
