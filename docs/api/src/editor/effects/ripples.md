# src/editor/effects/ripples.ts

The click ripple's pure half: the record, the cap, and the two questions the overlay asks of whatever the pointer landed on - "does this surface want a ripple at all?" and "which tint?". Kept out of [InterfaceEffects](InterfaceEffects.md) so both can be pinned by `ripples.test.ts` rather than by clicking around the editor. The tint rule in particular is a selector list: easy to break, impossible to notice breaking.

No React, no DOM writes - it only ever *reads* an element's ancestry via `closest`.

## RIPPLE_FROM

```ts
export const RIPPLE_FROM = 8
```

The ring's starting diameter in px.

## RIPPLE_TO

```ts
export const RIPPLE_TO = 56
```

The ring's final diameter in px. The span is drawn at **this** size in CSS and only ever `scale`d, so `RIPPLE_FROM / RIPPLE_TO` is the initial scale the overlay animates to `1`. That is what keeps a bloom to one composited transform and never a layout.

## RIPPLE_MS

```ts
export const RIPPLE_MS = 320
```

How long a ripple takes to grow to full size and fade to nothing. Fed to Motion as `RIPPLE_MS / 1000`.

## RIPPLE_CAP

```ts
export const RIPPLE_CAP = 6
```

How many ripples may be live at once. A **cap, not a rate limit**: a fast clicker still gets one ripple per click, but the seventh evicts the oldest. The eviction is why the overlay wraps its list in `AnimatePresence` - an evicted ripple exits through a fade rather than popping out of the DOM mid-bloom. Without the cap, a stuck pointer or a pointerdown storm could accumulate nodes.

## RippleTone

```ts
export type RippleTone = "rim" | "accent"
```

Which of the two tints a ripple carries. `"rim"` is the quiet neutral hairline - the default for every surface in the editor. `"accent"` is the app's red, reserved for controls that are already saying something. See [rippleTone](#rippletone-1) for the rule.

## Ripple

```ts
export interface Ripple { id: number; x: number; y: number; tone: RippleTone }
```

One live ripple. `x`/`y` are **client** coordinates: the overlay is `position: fixed; inset: 0`, so they drop straight into `left`/`top` with no measuring and no scroll correction. `id` is a monotonic counter owned by the overlay (a `useRef`, not a state), used as the React key and as the handle for [dropRipple](#dropripple).

## FX_OFF_ATTR

```ts
export const FX_OFF_ATTR = 'data-ui-fx="off"'
```

The opt-out marker, exported so the attribute is spelled once. Two surfaces carry it today:

- `.e-stage` in `stage/Stage.tsx` - the preview frame owns its pointer outright (click to add a zoom, click to aim, drag the reticle / the PiP handle / an arrange panel), and the one thing it must never do is paint chrome over the composited video, which has to show exactly what the export shows.
- `.e-tracks` in `timeline/Timeline.tsx` - the track stack is a drag surface (scrub, drag a pill, drag a handle). A bloom at the top of every drag would fire on the gesture's first frame rather than on a click.

New surfaces opt out where they are written, not by editing this file.

## suppressesRipple

```ts
export function suppressesRipple(el: Element | null): boolean
```

Is `el` inside (or itself) a surface marked `data-ui-fx="off"`? Walks ancestors via `closest`, so marking a wrapper covers everything drawn inside it.

A `null` target - a pointerdown that resolved to something that is not an `Element` - is **not** suppressed. That is the editor's own ground, which ripples like anything else.

## rippleTone

```ts
export function rippleTone(el: Element | null): RippleTone
```

The tint for a ripple landing on `el`: `"accent"` when it or any ancestor matches `.e-play, .e-export, .on, [aria-current]`, else `"rim"`.

**Ancestors, not the element itself**, because the pointer almost never lands on the button that carries the class - it lands on the `<svg>` glyph or the `<span>` label inside it. The four selectors are the editor's existing vocabulary for "this control is the hero, the commit, or currently engaged": the play button, Export, `.on` (trim pills, tool toggles, view mode, lane labels) and the rail's `aria-current` slot.

## pushRipple

```ts
export function pushRipple(list: readonly Ripple[], r: Ripple): Ripple[]
```

Append, evicting from the front once past `RIPPLE_CAP`. Returns a new array (the caller is a `setState`), oldest first.

## dropRipple

```ts
export function dropRipple(list: Ripple[], id: number): Ripple[]
```

Remove one ripple by id.

**Returns the same array reference when the id is not present.** The overlay calls this from `onAnimationComplete`, which fires for the *exit* animation too - so an already-evicted ripple reports completion a second time. Returning the identical reference is what makes React bail out of that `setState` instead of re-rendering the overlay for nothing.
