# src/editor/shell/PropertiesSlot.tsx

The `properties` editor type: the six inspectors, routed by `p.sel`.

## SelectedClip

```ts
export type SelectedClip =
  | { kind: "zoom"; zoom: Zoom }
  | { kind: "fx"; effect: EffectRegion }
  | { kind: "layout"; layout: LayoutSeg }
  | { kind: "cam"; move: CameraMove }
  | { kind: "cut"; cut: Cut }
  | { kind: "speed"; speed: Speed };
```

What one selection resolves to: the lane it belongs to, plus the clip itself, already typed. The
`kind` strings are `InspectorKind`'s, so the routing, the accent class and the timeline lane all
name a clip the same way.

## selectedClip

```ts
export function selectedClip(doc: EditDoc, sel: string | null): SelectedClip | null
```

The selection-to-inspector ladder, as a pure function: zoom / effect / layout segment / camera move
/ cut / speed span, in that order, by id (the last two from the time remap, T7). `null` for no
selection, for an empty string, and for a **stale** id that matches nothing - so deleting the
selected clip reads as "nothing is selected" rather than as "an inspector that failed to load".

**This is also the sidebar's existence test.** `ClassicShell` mounts the properties aside exactly
while this returns non-null and unmounts it otherwise, and this file renders exactly what it
returns, so the collapsed column and the empty inspector can never disagree. Being pure is what
makes that testable without mounting a shell: `PropertiesSlot.test.tsx` covers the collapsed cases
(no selection, stale id) and the arrived ones directly.

## PropertiesSlot

```tsx
export function PropertiesSlot({ p }: { p: SlotProps }): JSX.Element
```

**This is the only editor type selection touches.** Pulling the routing out in front of
`EditorPanels`' tab router is what makes selection and panels independent axes: a `panel` area
beside this one keeps showing whatever tab it was on when a pill is clicked, and the stage and
timeline never re-render for a selection at all. `EditorShell.test.tsx` pins that by holding the
stage's canvas node across a selection.

**There is no empty state any more.** With nothing selected this renders nothing, because the
sidebar around it is not mounted either (owner, 2026-09-14: "collapse the right inspector by default
when nothing is selected"). The old typographic sentence - "Select a zoom, layout, camera move,
effect, cut or speed span on the timeline." - and its `.e-insp-empty` rule are both gone: they
existed only because the column used to be permanently mounted and had to say something while idle.
The stage takes the width back instead, which is a better answer to "there is nothing to inspect"
than any sentence.

**Cut and speed spans (T7).** A `sel` matching a `doc.cuts` entry opens `CutInspector` (start, end,
Remove), one matching a `doc.speed` entry opens `SpeedInspector` (factor slider, start, end, Remove).
Both take `p.dur` and `p.applyOp` like every other inspector and close through the same
`p.setSel(null)`, so a cut selected by clicking its hatched span on the timeline behaves exactly like
a selected zoom pill. See `CutInspector.md` and `SpeedInspector.md`.

**Swap animation.** `AnimatePresence` keyed on `sel`, on the app's 0.16s content tween with the same
-8px x offset the panels use, so moving between two selected clips reads as one surface changing
rather than two surfaces cross-fading. `useReducedMotion()` drops it to a plain opacity change. The
arrival and departure of the column *itself* is a separate, outer `AnimatePresence` in
`ClassicShell` - this one only ever handles clip-to-clip.

**Close means deselect.** Every inspector's close control (and its Delete) calls `p.setSel(null)`,
which now collapses the sidebar. That is the same round trip Escape performs from anywhere in the
editor (see `keymap.md`), and the same one an empty click on the stage or the timeline performs.

**Class reuse.** The wrapper keeps `.e-panel-slot` so the inspectors inherit the sizing rules they
were written against; `.e-props-frost` (the shell's frosting element) overrides the fixed width so
it fills the 360px column. The inspectors are written against whatever width the wrapper gives them,
which is why `TimingRow` is two number fields wide and `ValueRow` is what does three - see
`InspectorShape.md`.

**It owns the inspector stylesheet.** This file imports `../inspectors/inspectors.css`, and is the
only thing that mounts an inspector, so that sheet loads with the editor without adding
inspector-only rules to `editor.css`'s shared vocabulary. What lives there: `.e-ihead` and friends
(the inspector header), `.e-isec` (the section rhythm and its hairline), `.e-ihero` (a section's one
big number), `.e-ivals` (the grouped value row), `.e-aimbtn`, `.e-preseg` and the curve editor's
box. What did **not** move out of `editor.css`: `.e-field`, `.e-field2`, `.e-fl`, `.e-seg`,
`.e-ghostbtn`, `.e-chip` and `.e-switchrow`, which the seven panels also use - the inspectors only
re-skin those under a `.e-insp` ancestor (planes instead of strokes), so the panels keep theirs
untouched. A `.css` file has no `docs/api` mirror of its own (the validator resolves a doc back to a
`.ts`/`.tsx`/`.rs` source), which is why it is documented here.
