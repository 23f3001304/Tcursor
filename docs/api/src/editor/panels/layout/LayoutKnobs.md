# src/editor/panels/layout/LayoutKnobs.tsx

Every knob of ONE layout, in two groups. Split out of `LayoutsPanel.tsx` so the panel file stays a flat read of its flow (picker, schematic, knobs, reset, presets) and this one stays a flat read of the knob matrix.

## LayoutKnobs

```tsx
export function LayoutKnobs({ mode, ma, onChange }: {
  mode: ModeKey;
  ma: ModeAppearance;
  onChange: (next: ModeAppearance) => void;
}): JSX.Element
```

### Props

- `mode: ModeKey` - which of the five layouts is being edited. Only used to decide which controls exist; the values all come from `ma`.
- `ma: ModeAppearance` - that layout's current appearance block.
- `onChange: (next: ModeAppearance) => void` - called with the WHOLE next block on every change. This component holds no state of its own, which is what lets `LayoutsPanel` turn each change into exactly one `saveDocSettings` call and therefore one undo step.

### Which controls appear

Straight off the existing per-layout matrix in `src/hud/preferences/appearanceFields.ts` - `MODE_SLIDERS`, `MODE_HAS_SHAPE`, `MODE_HAS_CORNER` - which is the **same** matrix the HUD's `SettingsAppearance` reads. That is the point: a layout can never grow a control in one editor and not the other, and the two can never disagree about which knob a layout even has.

**Screen** group - `pad`, `screen_size`, `screen_radius`, each present only where `MODE_SLIDERS[mode]` lists it (Camera-only, for instance, has padding and no screen scale; Presenter has padding and a radius).

**Camera** group - shown when `MODE_HAS_SHAPE[mode]`, so Screen-only (which has no webcam at all) simply has no second group:

- `cam_size`, when the matrix lists it. Presenter does not: its webcam fills its half of the frame by construction.
- **Shape** (`Segmented`: Circle / Rounded / Rect), with **Corner Roundness** directly under it and only while the shape is `rounded` - the switch above what it enables, the panel pass's rule.
- **Aspect** (`Segmented`: Square / 16:9).
- **Dock Location** (`Segmented`: BL / BR / TL / TR) only where `MODE_HAS_CORNER[mode]`, which today is the `screen` bubble layout alone. The labels stay the short pair-of-letters the control has always used - what fits four-up at 320px - with the full name (`Bottom left`) as each segment's `title`.
- **Margin X / Margin Y** as a two-up `.e-two` row, and only where the matrix lists them (the bubble layouts). No heading over the pair: each slider's own readout row already names it.
- The **ring** - `CameraRingField`, unchanged, reused verbatim from the old Camera panel (switch, then width and colour when on).

### Notes

- `CORNER_OPTS` moved here from `CameraPanel.tsx` along with the controls it labels.
- `SCREEN_KNOBS` / `CAM_KNOBS` are the orderings, not the gates: membership is still `MODE_SLIDERS`. `cam_radius` is in neither, because it is rendered under the shape that makes it meaningful rather than in slider order.
- Every value is a fraction of the canvas, rendered as a percentage (`pct`), which is why a look saved on a 1080p recording applies unchanged to a 4K one.

### Used by

- `src/editor/panels/layout/LayoutsPanel.tsx`.
