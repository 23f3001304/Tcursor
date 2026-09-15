# src/editor/controls/fields/Segmented.tsx

A row of exclusive choices, all of them visible at once. Added by the panel pass (spec `2026-09-13-panel-design-benchmark.md`, section (b) point 2: "segmented controls for exclusive states, never dropdowns").

**When to use it instead of `Picker`.** Two to four exclusive states whose labels are one short word each. Everything longer stays a dropdown, because a four-up segmented row inside a 320px panel gives each label about 66px and an ellipsised segment is worse than a dropdown. The call sites that qualified: cursor style (System / Enhanced / Hidden), background kind (Wallpapers / Color / Gradient), webcam shape (Circle / Rounded / Rect), aspect (Square / 16:9) and dock location (BL / BR / TL / TR). The ones that did not: the AI engine list, ripple style (seven), spotlight mode (six) and video FX mode (four, but two words each).

**Props are `Picker`'s.** Same `value` / `options` / `onChange` / `ariaLabel` shape, so converting a qualifying dropdown is a component-name swap and nothing else. `options[].title` becomes each segment's tooltip, which is where a longer description goes once the label has been shortened to fit.

## segmentedNextIndex

```ts
export function segmentedNextIndex(key: string, index: number, length: number): number | null
```

The option index an arrow, `Home` or `End` press should move the selection to, or `null` for any other key.

`ArrowRight`/`ArrowDown` step forward, `ArrowLeft`/`ArrowUp` back, and both **wrap** - unlike `pickerNextIndex`, which clamps. *Why the difference:* a listbox can be long, so running off the end should stop; a 2-to-4 segment row is short enough that wrapping reaches any segment faster than reversing does, and it is what the WAI-ARIA radio-group pattern specifies. From "no segment matches the value" (`index < 0`) it steps from the first segment.

### Behaviors

- `steps forward and back, wrapping at both ends`.
- `treats the vertical arrows the same as the horizontal ones`.
- `jumps to the ends on Home/End and ignores everything else`.
- `starts from the first segment when nothing matches the value yet`.

## Segmented

```tsx
export function Segmented<T extends string>({ value, options, onChange, ariaLabel, columns }: {
  value: T;
  options: { value: T; label: string; title?: string }[];
  onChange: (v: T) => void;
  ariaLabel?: string;
  columns?: number;
}): JSX.Element
```

### Props

- `columns?: number` - lay the segments out as a grid of this many columns instead of one row. For four short labels that read better 2x2 than four-up. No call site needs it today; it exists because `CORNERS` is one label-length change away from wanting it.

### Accessibility

The wrapper is `role="radiogroup"` with `aria-label`; each segment is a `role="radio"` button with `aria-checked`, so exactly one segment is ever checked. Only the checked segment is in the tab order (`tabIndex` 0, the rest -1) - the radio-group convention: Tab reaches the group, arrows move within it. A key press both moves the selection and focuses the segment it moved to, so the roving tabindex stays under the user's focus.

### Motion

The active segment's raised plane is one `motion.span` with a `layoutId` (unique per instance via `useId`), so the plane *glides* to the segment that was picked instead of blinking there. Spring 500/30, the app's press spring. Under `useReducedMotion` the `layoutId` is dropped, which turns the glide into an instant move without changing anything else about the control.

### Styling

`.e-segmented` / `.e-segment` / `.e-segment-slot` / `.e-segment-label` in `controls/controls.css`. The track is a sunken plane (`--e-bg`) and the active slot a raised one (`--e-raised`) - deliberately NOT an accent fill, which would spend the panel's one accent on something that is not a selection marker in the sense the benchmark means (an accent fill on a three-wide row reads as a filled tab bar, not as state).

### Used by

- `src/editor/panels/cursor/CursorPanel.tsx` - cursor style.
- `src/editor/panels/background/BackgroundPanel.tsx` - background kind.
- `src/editor/panels/camera/CameraPanel.tsx` - webcam shape, aspect, dock location.
