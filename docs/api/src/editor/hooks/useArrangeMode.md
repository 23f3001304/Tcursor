# src/editor/hooks/useArrangeMode.ts

Owns stage arrange mode for `Editor`: which layout segment's panel frames the stage is showing, and the ways in and out. The decision itself is `nextArrangeMode` (`arrangeMode.md`); this hook only supplies the events and derives the segment.

## useArrangeMode

```ts
export function useArrangeMode(doc: EditDoc | null, sel: string | null, io: {
  setSel: (id: string | null) => void; timeMsRef: { current: number }; onSeek: (ms: number) => void;
}): { arrangeSeg: LayoutSeg | null; arrangeOn: boolean; onArrange: () => void; onSel: (id: string | null) => void }
```

`io` is read through a ref at call time, so `onArrange`/`onSel` stay referentially stable across renders (both are props of `React.memo`'d hot-tree components).

`arrangeSeg` is non-null exactly when the mode is on AND the segment still exists - `Stage` takes it as its whole arrange-mode signal, so there is no way to render the overlay for a segment that has gone away. `arrangeOn` is what `LayoutInspector` reads to swap its "Arrange on stage" button for the live hint, and what `Editor` folds into `aimMode`.

### Entry and exit

- **Selecting a layout pill** on the timeline enters the mode, through the returned `onSel` - which `Editor` gives to `Timeline` in place of a bare `setSel`. It has to dispatch the select event itself, not just set the selection: after an Escape `sel` still holds that segment, so re-clicking the same pill sets no new state and a change-keyed effect would never fire - the pill would be a dead control, and the brief names it as an entry gesture. The `[selSegId]` effect still covers every OTHER way the selection moves (rail tab switch, Delete, an inspector's close button); the two paths emit the same event, and `nextArrangeMode` returns the CURRENT mode object for a `select` that changes nothing, so the second dispatch really does bail out rather than re-rendering `Editor` for an equal-but-new object. The selection is only treated as an arrange target when it is actually a layout segment id (`doc.layout`), so selecting a zoom, effect or camera keyframe exits exactly like a deselect.
- **`onArrange`** (the inspector button) re-enters after an Escape, using the remembered segment.
- **Escape** exits, keeping the segment remembered. It is a LOCAL `keydown` listener, live only while the mode is on, rather than a new action in `keymap.ts` - the same shape `DirectorScrim`, `CameraLane`'s curve popover and the dialogs already use for their own dismiss key. Keeping it local means a global Escape never acquires an editor-wide meaning that every other surface would then have to opt out of, and the listener does not exist at all when nothing is being arranged.
- **The segment disappearing** (deleted, or a doc reload without it) exits outright, so the inspector's re-enter button can never point at a segment that is no longer there.

### Seek on entry

Entering seeks the playhead into the segment when it is outside the span, to `arrangeSeekMs` - `start_ms + transition_ms`, clamped inside `[start, end)`, so the user lands on the SETTLED layout rather than mid-fade. Keyed on the entry itself (`mode.on` + `mode.segId`), so scrubbing back out afterwards is left alone.

This is correctness, not convenience: `layoutAt` only selects a segment while the playhead is inside its span, so from outside it the live drag draft has no effect on the composite at all, and the brief's "dragging updates a local draft consumed by the composite loop for THIS segment" would silently not hold. Same shape as `ZoomInspector`'s seek-into-span. The frames themselves still render the segment's steady state, not the frame at the playhead - see `arrange/useArrangeDrag.md`.

### Exclusivity

Arrange, aim and camera Move all claim the same canvas pointer. `Editor` computes `aimMode = aimOn && !moveMode && !arrangeOn && !!aimPoint`, and `Stage` suppresses click-to-zoom, the aim reticle and the Move drag handle for as long as `arrangeSeg` is non-null - and also SUPPRESSES (never discards) any unsaved Move-mode camera draft while it lasts, because `frameCamLayout` gives `camDraftRef` precedence over the base layout rect, which is exactly where the arrange draft lives (see `frameCam.md`'s `activeCamDraft`). Selection alone already makes arrange and aim mutually exclusive (one needs a layout segment selected, the other a zoom), but the gate is explicit so a future path that lets both coexist cannot silently produce two live stage modes.
