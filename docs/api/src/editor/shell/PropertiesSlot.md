# src/editor/shell/PropertiesSlot.tsx

## PropertiesSlot

```tsx
export function PropertiesSlot({ p }: { p: SlotProps }): JSX.Element
```

The `properties` editor type: the six inspectors, routed by `p.sel`.

### Behavior

**This is the only editor type selection touches.** The routing (zoom / effect / layout segment / camera move / cut / speed span, in that order, by id - the last two from the time remap, T7) is the same chain that used to sit in front of `EditorPanels`' tab router. Pulling it out here is what makes selection and panels independent axes: a `panel` area beside this one keeps showing whatever tab it was on when a pill is clicked, and the stage and timeline never re-render for a selection at all. `EditorShell.test.tsx` pins that by holding the stage's canvas node across a selection.

**Empty state is typographic.** With nothing selected: one sentence, hint type, `--e-dim`, capped at 26 characters per line and centred in the column (`.e-insp-empty`) - "Select a zoom, layout, camera move, effect, cut or speed span on the timeline." No icon, no illustration, no bordered card. This is the state the panel sits in most of the time now that it is a permanently mounted right sidebar rather than something a selection summons, so it has to read as a calm label, not as a panel that failed to load.

**Cut and speed spans (T7).** A `sel` matching a `doc.cuts` entry opens `CutInspector` (start, end, Remove), one matching a `doc.speed` entry opens `SpeedInspector` (factor slider, start, end, Remove). Both take `p.dur` and `p.applyOp` like every other inspector and close through the same `p.setSel(null)`, so a cut selected by clicking its hatched span on the timeline behaves exactly like a selected zoom pill. See `CutInspector.md` and `SpeedInspector.md`.

**Swap animation.** `AnimatePresence` keyed on `sel ?? "none"`, on the app's 0.16s content tween with the same -8px x offset the panels use, so moving between two selected clips reads as one surface changing rather than two surfaces cross-fading. `useReducedMotion()` drops it to a plain opacity change.

**Close means deselect.** Every inspector's close control calls `p.setSel(null)`, which collapses the area back to its strip in the default workspace. That is the same round trip Escape now performs from anywhere in the editor (see `keymap.md`).

**Class reuse.** The wrapper keeps `.e-panel-slot` so the inspectors inherit the sizing rules they were written against, plus `.e-area-props`; the area/sidebar CSS overrides the fixed width, the radius and the shadow, since the frame around it owns the elevation. The inspectors are written against whatever width the wrapper gives them (300px in the classic right sidebar), which is why their Timing rows are two number fields wide and never three - see `InspectorShape.md`.

**It owns the inspector stylesheet.** This file imports `../inspectors/inspectors.css`, and is the only thing that mounts an inspector, so that sheet loads with the editor without adding inspector-only rules to `editor.css`'s shared vocabulary. What moved there out of `editor.css`: `.e-aimbtn`, `.e-del` and the old `.e-sec-hint` (no caller outside the inspectors, so they moved wholesale, and `.e-del` became a quiet ghost instead of a filled red slab). What did **not** move: `.e-field`, `.e-field2`, `.e-fl`, `.e-seg`, `.e-ghostbtn`, `.e-chip` and `.e-switchrow`, which the seven panels also use - the inspectors only re-skin those under a `.e-insp` ancestor (planes instead of strokes), so the panels keep theirs untouched. A `.css` file has no `docs/api` mirror of its own (the validator resolves a doc back to a `.ts`/`.tsx`/`.rs` source), which is why it is documented here.

In the classic layout `ClassicShell` renders this in the one panel slot while `sel` is set and `EditorPanels` otherwise, which is the old "selecting a pill shows its inspector" behaviour.
