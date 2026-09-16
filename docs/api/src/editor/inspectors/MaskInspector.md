# src/editor/inspectors/MaskInspector.tsx

The inspector a selected mask region opens: Timing, Region, Look and Fades for a Blur, Pixelate or Highlight.

**Why this is a separate file and not a branch in `EffectInspector`.** `EffectInspector` is the spotlight's, at 186 lines of its 200 budget, and its whole body is the spotlight's six modes and their override fields - a mask shares none of them. Adding a kind branch there would have blown the budget, forced the spotlight's own controls behind a condition, and put the two effect families' edits in one file for the rest of their lives. `PropertiesSlot` dispatches on `isMask` instead, so the spotlight inspector is untouched by this feature and neither file has to know the other exists.

## MaskInspector

```tsx
export function MaskInspector({ effect, dur, settings, onApply, onClose }: {
  effect: EffectRegion; dur: number; settings: Settings;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

The same prop shape as `EffectInspector` minus `onDimCamera`: a mask never dims the webcam panel, so it needs no settings write and takes no settings callback. Every edit is one `update_effect` op through `onApply`; the component holds no draft state of its own.

### The sections

| Section | Controls |
|---|---|
| Timing | `TimingRow` over `start_ms` / `end_ms` |
| Region | X, Y, Width, Height as `NumberField` percentages, plus the drag hint |
| Look | the kind's own strength slider, Soft edge, Corners |
| Fades | Fade in, Fade out in seconds, each capped at the region's own span |

**The Region numbers are percentages of the RECORDED CANVAS, not of the stage.** The stored `rect` is `[x, y, w, h]` in 0..1 of the canvas that was recorded, which is what makes a mask hold its place through a zoom, a layout change and a different export size. So 50% means the middle of the recording, not the middle of what is currently on screen, and the two differ under any zoom. X and Y may be 0; Width and Height are floored at 1% so a rect can never be dragged to nothing through the fields.

**The Look section is kind-aware, one control deep.** Highlight gets a Dim slider (20 to 90%) defaulting to the project's `clickfx.spotlight_dim`, so the two dimming effects agree unless the user says otherwise. Blur and Pixelate share one slider over `strength` (0.2 to 12% of the output height) whose label and aria label read "Blur radius" or "Pixel size" - the same field means a different thing and must say so. Soft edge (`feather`) and Corners (`roundness`) apply to all three.

Every slider takes `accentColor="var(--e-fx)"`, the FX lane's accent, because a mask lives on that lane.

### Look note

The Region row is `.e-field2 .e-maskregion`. `.e-field2` is a single-row flex built for a PAIR, and four fields in it would squeeze each `NumberField` to a quarter width; `.e-maskregion` (in `inspectors.css`) adds `flex-wrap` and a 50% basis so the four wrap to a 2x2 block instead. Tokens only, no colour.

### Used by

- `src/editor/shell/PropertiesSlot.tsx` - the `mask` branch, reached when `selectedClip` finds an effect and `isMask` says it is not a spotlight.
