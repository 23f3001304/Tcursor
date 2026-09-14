# src/editor/inspectors/zoomFeel.ts

The Feel row's data, in its own module rather than inline in `ZoomInspector.tsx`: three named feels
and the two pure functions that read a zoom against them. Pulling them out is what lets the row be
unit-tested without mounting a panel (`zoomFeel.test.ts`), and it keeps the inspector under the
200-line cap now that Framing carries a hero value and Timing carries a grouped row.

A feel is **how the zoom moves**, which is exactly three fields: the in duration, the out duration
and the easing. Scale is deliberately not one of them - it is the Framing section's hero value, the
thing the owner's own example calls out first ("Scale 2.8x"), and a preset that silently re-framed
the shot every time you asked for a different pace would be the opposite of a feel.

## FeelPreset

```ts
export interface FeelPreset { name: string; zoom_in_ms: number; zoom_out_ms: number; easing: string }
```

## FEEL_PRESETS

```ts
export const FEEL_PRESETS: FeelPreset[]
```

The three, in order, quiet to loud:

| name | zoom in | zoom out | easing |
| --- | --- | --- | --- |
| Subtle | 450ms | 550ms | `ease_in_out` |
| Balanced | 350ms | 450ms | `smooth` |
| Punchy | 200ms | 300ms | `spring` |

Durations shorten monotonically down the list and every preset carries a different curve, so the
row is about feel and not only about speed; `zoomFeel.test.ts` pins both properties, since a later
edit that gave two presets the same easing would leave a row of three planes doing two things.

Balanced is the shape a zoom is created at, which is why a freshly added zoom opens with that plane
already lit. The easing keys are `timeline/curveGlyphs.ts` names, the same wire names the ops and
the Rust `easing_from` understand, so a preset is reproducible in the export and not a preview-only
nicety.

## activeFeel

```ts
export function activeFeel(z: Pick<Zoom, "zoom_in_ms" | "zoom_out_ms" | "easing">): string | null
```

The preset a zoom currently matches on **all three** fields, exactly, or `null`.

`null`, not `"Custom"`: the segmented row renders with **no plane selected** in that state, so
nudging Zoom in by one step visibly un-lights the row instead of leaving a preset falsely on or
inventing a fourth option that cannot be picked. The Feel section's own right-aligned readout is
where the word "Custom" is said (`ZoomInspector`).

`scale` is not in the `Pick`, so a zoom framed at 3.9x still reads as Balanced if it moves like
Balanced.

## feelPatch

```ts
export function feelPatch(name: string): Pick<Zoom, "zoom_in_ms" | "zoom_out_ms" | "easing"> | null
```

The `update_zoom` patch a preset name applies, or `null` for a name that is not a preset (so an
unknown key from the row applies nothing rather than writing an empty op). One click is therefore
one op, one undo step, and one preview bump - not three.
