# src/editor/inspectors/CutInspector.tsx

The selected cut's properties: the two edges, in clip seconds, and Remove. Routed to by `PropertiesSlot` whenever `sel` matches a `doc.cuts` entry, in the same register as every other inspector - the same `PanelHeader`, the same `TimingRow`, the same `RemoveButton` last (`InspectorShape.md`).

Numeric only, on purpose: a cut is a range of removed time, not a thing with a look, and its hatched span on the timeline deliberately has no drag handles (`CutOverlay.md`), so this is where a millisecond is reachable. It is therefore the one inspector with a single section, **Timing** - there is no look and no motion to give it a second.

## CutInspector

```tsx
export function CutInspector({ cut, dur, onApply, onClose }: {
  cut: Cut; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `cut: Cut` - the selected cut, found by id in `doc.cuts`.
- `dur: number` - the clip's duration, the End field's ceiling.
- `onApply` - every control applies one `update_cut` (or `remove_cut`), which persists the doc and bumps the preview, so an edit is visible immediately.
- `onClose` - drops the selection (`p.setSel(null)`), the same "Deselect" every inspector's header button does.

### Behavior

**Clip time, not output time.** Start and End are the clip's own clock - the clock every pill on the timeline sits on and the clock the `update_cut` op takes - so a cut's numbers keep meaning the same thing after another cut lands earlier in the clip. The transport's readout is where output time is shown.

**The header says where, the section says how much.** The lede is the shared `spanLede` ("0:01.0 to 0:02.5, 1.5 s") and the Timing section's right-aligned readout is "removes 1.5 s", both live. The hint under the fields carries the consequence: everything after it moves earlier in the export, and the recording itself is untouched.

**Remove.** `remove_cut` then `onClose`, matching every other inspector's Remove exactly. The same op is what Delete on a selected cut applies (`useEditorKeymap`), so the keyboard and the button are one path.

### Notes

- The Rust ops normalise after every edit: a cut dragged to zero length is dropped, and two cuts that end up overlapping or touching merge into one (the earlier id survives). Both can therefore change the doc more than the single field the user edited, which is why the inspector reads the cut back out of the doc every render rather than holding a draft.
