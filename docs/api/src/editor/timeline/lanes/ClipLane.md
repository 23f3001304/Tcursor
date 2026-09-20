# src/editor/timeline/lanes/ClipLane.tsx

The Clips lane: one pill per `doc.clips` entry, drawn through the same `RegionRows` every region lane shares. Renders only while `useTimelineLanes.tsx` decides there is more than one clip - see its own "Clips lane" note.

## ClipLane

```tsx
export const ClipLane: React.MemoExoticComponent<(props: {
  clips: Clip[]; map: TimeMap; dur: number; sel: string | null;
  onSel: (id: string | null) => void;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  track: React.RefObject<HTMLDivElement | null>;
}) => JSX.Element>
```

### Props

- `clips: Clip[]` - `doc.clips` itself, in OUTPUT order. Passed straight through rather than the `clipRegions`-sorted array, because `dropIndex` needs the list's own index order to compute a `move_clip` target.
- `map: TimeMap` - `clipOutMs` needs it to label each pill with the time the export will actually spend on that clip (`clipModel.md`).
- `dur` / `sel` / `onSel` - the same clip-duration and selection contract every other lane takes.
- `onApply` - persists an edit op; see "The two ops" below.
- `track` - the timeline's scroll/track element, handed to `useClipDrag` for its pointer-delta-to-ms math, exactly as `useRegionDrag` takes it.

### The two ops it commits, and the one it never does

- A body drag (`useClipDrag`'s `onReorder`) resolves a target index with `dropIndex(clips, id, atMs)` and, when one exists, commits `{ op: "move_clip", id, to_index }`. No target (the release point isn't over another clip) commits nothing.
- An edge drag (`onRetime`) commits `{ op: "update_clip", id, src_in_ms, src_out_ms }` - both bounds always, the untouched one at its own current value, the same shape `TimeLane`'s `update_speed` commit already uses. It looks the clip up in `clips` by `id` first and commits nothing when both bounds already match what is there (fixed in Batch 4 T8's fix round 1: a drag that clears the pointer-distance threshold in raw pixels but clamps or rounds back to exactly where it started used to still send a no-op `update_clip`).
- It never commits `remove_clip`. Deleting a clip collapses the recording back together, which reads as a bigger decision than a drag should make by accident - that action belongs to the clip's inspector and to the keymap's Delete handling on the current selection, neither of which this task builds. Selecting a clip here (`onSel`, wired to every pointer-down) is what a later inspector will act on.

### Behavior

`regions` is `clipRegions(clips, map)`, memoised on `[clips, map]`. `useClipDrag(regions, dur, track, onReorder, onRetime, onSel)` supplies the live drag draft and the `beginDrag` starter `RegionRows` wires to the pill body and both edge handles. Row count is `Math.max(1, ...regions.map(r => r.layer + 1))` - the same "at least one row" arithmetic `useTimelineLanes.tsx` uses to size the lane's gutter label before this component ever mounts.

`RegionRows` gets `rowClass="e-cliprow"` / `blkClass="e-clipblk"`, `renderLabel={clipLabel}`, `extraStyle={clipExtraStyle}` (the left-edge transition ramp), and a `titleOf` that names the clip's order and both gestures ("Drag it onto another clip to change the export order, drag its edges to retime it") - the one lane-specific hint text, since nothing else here needs a `title` to be usable.

### Used by

`src/editor/timeline/useTimelineLanes.tsx` - pushed first into the lane stack, above Time.
