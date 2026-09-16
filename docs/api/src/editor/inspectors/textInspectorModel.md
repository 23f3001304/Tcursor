# src/editor/inspectors/textInspectorModel.ts

The tables and the two derivations `TextInspector` reads, kept out of the component so they can be tested without mounting anything.

## ANCHOR_GRID

```ts
export const ANCHOR_GRID: TextAnchor[];
```

The nine anchors in READING ORDER, three by three: top left to top right, then the middle row, then the bottom. `TextInspector` lays them into a 3x3 CSS grid in exactly this order, so the control's shape is the frame's shape and picking a corner means pointing at that corner.

The order is pinned by a test rather than left to the enum, because the enum's declaration order and the grid's reading order agreeing is a coincidence worth keeping deliberate: reordering `TextAnchor` in `editText.ts` for any other reason must not silently transpose this grid.

### Behaviors

- `lays the nine anchors out in reading order, three by three`.

## ANIM_OPTIONS

```ts
export const ANIM_OPTIONS: readonly { value: string; label: string }[];
```

The four animations for the In and Out dropdowns, in the order the spec lists them: Fade, Slide, Pop, Typewriter.

Both dropdowns take the same four. Typewriter as an OUT animation is legal and does something sensible - `lay_one` gives a typewriter full alpha until the last quarter of the out ramp - so there is no reason to split the list in two.

## SIZE_OPTIONS

```ts
export const SIZE_OPTIONS: readonly { value: string; label: string }[];
```

The five sizes as a `Segmented` row. The labels are the wire names uppercased (XS to XL) rather than pixel figures, because the actual size is a fraction of output height (`TEXT_SIZE_FRACS`) and a pixel figure would be a lie at any other export resolution.

## slideIsPointless

```ts
export const slideIsPointless: (pos: TextAnchor) => boolean;
```

True only for `mid_center`.

A slide travels from the edge the item is anchored to (`lay_one`'s `slideDir`), and the centre has no edge - its direction vector is `(0, 0)`, so a centred slide is arithmetically identical to a fade. The inspector does not REMOVE the option when this is true: the item may already be set to `slide`, and a dropdown whose current value is missing from its own list is worse than one that explains itself. Instead the option keeps a `title` saying why it does nothing there, and a `Hint` under the section says the same in prose.

*Why not `disabled`:* `Picker`'s option shape has no `disabled` field, and adding one would mean editing a shared control this track has no claim on.

### Behaviors

- `greys Slide out at the centre, where it has no direction` - true for `mid_center`, false for all eight others.

## textGraphInput

```ts
export function textGraphInput(item: TextItem): GraphInput
```

The item as the M3 motion graph wants it: its own span, its two ramps and a peak of 1.

- `lane: "text"` - picks `--e-text` for the curve and "Text" for the graph's aria label. `baseOf("text")` is 0, so the curve rests at nothing and rises to full, which is exactly what a text item's alpha does. Only `zoom` rests at 1.
- `peak: 1` - the graph is the item's ALPHA, and alpha's maximum is 1 by definition. There is no per-item strength to plot the way a zoom has a scale.
- `rampIn` / `rampOut` - `item.easing` with `in_ms` and `out_ms`. Both ramps take the SAME easing because a `TextItem` stores one (`easing`), not the in/out pair a `Zoom` carries. If the owner ever wants them split, the field is what has to grow first, not this function.

### Behaviors

- `feeds the motion graph the item's own span and ramps` - the span, both ramps, the peak and the lane, each asserted.
