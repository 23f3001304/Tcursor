# src/editor/inspectors/TextInspector.tsx

The inspector a selected text item opens: Timing, Content, Look and Motion, in that order.

## TextInspector

```tsx
export function TextInspector({ item, dur, onApply, onClose }: {
  item: TextItem; dur: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### The sections

| Section | Controls |
|---|---|
| Timing | `TimingRow` for start and end, plus a hint that the lane pill moves it |
| Content | `TextContentField`: the textarea, the clearable second line, and the Kind `Picker` |
| Look | Style `Picker`, the 3x3 anchor grid, Nudge X and Nudge Y sliders, Size `Segmented` |
| Motion | In and Out `Picker`s with a `NumberField` of seconds each, the M3 `MotionField`, and the hints |

Every control is from the standing vocabulary (`Controls.tsx`, `InspectorRows.tsx`, `InspectorShape.tsx`): plain dropdowns, a segmented row, sliders, number fields. No card editor and no drag-to-reorder editor (`classic-layout-kept-shell-vetoed`).

### Every write is one `upd`

```ts
const upd = (patch: Patch) => void onApply({ op: "update_text", id: item.id, ...patch });
```

One op shape for the whole inspector, including every keystroke in the textarea. That is deliberate and it is what makes typing usable: `useEditHistory` coalesces same-shape ops within 400 ms, so a burst of typing collapses into ONE undo step rather than one per character. It is also why the textarea is fully controlled off `item.text` and commits on `change` rather than on blur the way `CaptionTextField` does - a caption's text carries word timings that must not be re-derived mid-word, a text item's does not.

`onKeyDown` stops propagation on both text inputs, so typing a `t` or an `s` into a title cannot also fire the editor's own shortcuts. (`resolveKeyAction` already stands down inside an `INPUT`/`TEXTAREA`; this is the second lock on the same door, and it costs nothing.)

### The anchor grid

Nine dots in a 3x3 grid, over `ANCHOR_GRID`, built on the shared `SegRow` rather than on a bespoke control - so the keyboard behaviour, the pressed state and the tick are the ones every other segmented row already has, and only the LAYOUT is new (`.e-anchorgrid` in `inspectors.css` turns the row's flex into a grid). The dot is the whole label, which is why each option's `label` is empty and `visual` carries the dot.

### Nudge is two sliders, and deliberately not an on-stage drag

A text item lives in OUTPUT-FRAME space, not on the picture: it does not track the cursor, it does not follow a zoom, and it is not anchored to anything in the recording. A drag box on the stage would imply all three (spec 5.7). Two sliders, each a percentage of the frame, say what is actually true - the offset is a fraction of the output, so it survives an aspect change and an export at any resolution.

### The header and the delete label

The header title is the KIND's label ("Lower third"), not a generic "Text", and the delete button reads `Delete lower third`. A project with a title, two lower thirds and a callout in it should be tellable apart from the inspector alone.

### The shell's accent

`InspectorShell kind="caption"`. `"text"` is not an `InspectorKind`, and rather than widen that union for a colour, the inspector borrows the caption accent: both are words laid over the picture, so the borrowed hue is not wrong. The Text LANE has its own `--e-text` orchid; if the owner's look pass wants that in the inspector too, `"text"` joins `InspectorKind` then and this line changes.

### The three hints

- On a typewriter: the whole line is revealed over the In duration, which is the one thing about that animation that is not visible from its name.
- At the centre anchor: Slide has no edge to come from there, so it reads as a fade. See `slideIsPointless`.
- Always: captions are drawn last and nothing covers them, so a line parked at the bottom centre will sit under one. This is spec 4's fit point 1 surfaced to the user rather than worked around - the renderer deliberately does not move anything to avoid the collision, because silently relocating what the user positioned is worse than letting them see it.

### Used by

- `src/editor/shell/PropertiesSlot.tsx` - the `"text"` branch of the selection ladder.
