# src/editor/inspectors/LayoutInspector.tsx

Left-panel inspector for the selected timeline layout segment. Timing and transition controls apply `update_layout_seg`; the arrangement controls apply `set_arrangement` / `clear_arrangement`. Everything goes through `onApply`, which persists the doc and bumps the preview, so edits show live.

## LayoutInspector

```tsx
export function LayoutInspector({ seg, dur, presets, arrangeOn, onApply, onArrange, onClose }: {
  seg: LayoutSeg; dur: number; presets: LayoutPresets | null; arrangeOn: boolean;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onArrange: () => void; onClose: () => void;
}): JSX.Element
```

### Controls

- **Header thumbnail** (T34 L4) - `PanelHeader`'s `thumb` slot renders `LayoutThumb` (`../timeline/LayoutThumb.tsx`) at 48x28 from the SAME `panels` (`resolvedPanelsFor(seg, presets)`) this file already computes for the visibility switches below, so the schematic and the switches can never disagree about what's actually shown.
- **Start from** (T34 L3, replaces the old Preset picker) - `Camera` / `Presenter` / `Screen only` / `Camera only`, as a chip row. `"screen"` stays deliberately absent for two reasons now: it IS the empty default (reached by deleting the pill or leaving a gap), and it is the one layout the timeline hides as a pill - starting from it would make the very segment being edited vanish from the track.
- **Arrange on stage** - enters stage arrange mode for this segment (`useArrangeMode.md`). Replaced by a `--e-dim` hint line while the mode is already on, since selecting the pill enters it anyway and the button would be a no-op.
- **Custom arrangement, based on X** + **Reset to preset** - shown only once the segment carries an `arrangement`. Reset is `clear_arrangement`, the one way back to a plain preset-driven segment.
- **Show screen** / **Show webcam** - per-panel visibility. Re-showing restores the panel at whatever it currently RESOLVES to, which for one the arrangement hid is its provenance preset's own placement (L1: a hidden panel still resolves to a real rect, just at alpha 0). The switch for the LAST visible panel is DISABLED (`Switch`'s new `disabled` prop, with a "One panel has to stay visible" title) rather than live-but-ignored: `set_arrangement` rejects a write that would blank the frame, and a rejected op still resolves, so a switch that only early-returned would look like it worked and would leave a phantom undo step behind.
- **Start / End** - the segment's `[start, end)` span, in seconds.
- **Transition** + its curve - the ENTRY cross-fade, which STARTS at `start_ms`.
- **Exit transition** + **Exit Curve** - the exit cross-fade, which COMPLETES at `end_ms`. `0` is a hard cut; a segment created by `add_layout_seg` starts at 350ms, the same as its entry (`NEW_LAYOUT_TRANSITION_MS`, `api.md`), and only a doc saved before this field existed loads at `0`. The curve row only appears once the duration is non-zero, since `easing_out` means nothing at `0`.
- **Delete layout**.

### What "Start from" actually writes

Two ops, back to back: `update_layout_seg { layout }` then `set_arrangement { screen, cam }` fed from that preset's own `arrangement` (`LayoutPresetDto.arrangement`, derived in Rust by `arrangement_of_preset` so the conversion has exactly one definition). Both land inside the history's 400ms coalesce window, so it is ONE undo step (`useEditHistory.md`).

The `layout` name has to move too, not just the poses: it is what selects the appearance block the poses resolve against - radius, ring, webcam shape - so poses from one preset resolved under another preset's block would not be the preset the chip names.

**It is a starting point, not a binding.** After the click the segment carries its OWN arrangement, and the preset name survives only as editable provenance ("based on Presenter"). The hint under the row says so, and points at the way back (*"Reset to preset puts it back on a plain preset"* - chip then Reset is two clicks to any plain preset, since the chip moves `seg.layout` first). It deliberately does not claim the two render identically: under `CamAspect::Wide`, converting one of the big-camera presets legitimately widens the webcam to the user's configured 16:9, because `scene::resolve` draws those three presets as a square regardless of the configured width while an arrangement honours it. That divergence is a pre-existing quirk of the preset path (documented and pinned by L1's `a_wide_webcam_widens_the_big_camera_presets_which_resolve_draws_square`), and the arrangement is the side that is right - so the UI must not promise pixel-identity it would not deliver.

### What the Exit section actually does

The exit blends from this segment toward whatever the track resolves after it - the next segment if the two are gapless, else the base `screen` layout - landing on that pose exactly at `end_ms`.

Its visible effect is narrower than it looks, and deliberately so: when a gapless successor has its own entry transition, that entry WINS the overlap and the exit stands down (see `export/scene/layout.md`). Only one blend runs at a time, so back-to-back segments stay seamless. The exit therefore shows up when a segment falls back into a GAP, or hands off to a successor that hard-cuts in - which are exactly the two cases that used to pop.

Both curve rows are the shared `CurveEditor`, so an exit can be hand-drawn as a custom `cubic(...)` just like an entry.

### Notes

- Panel visibility and the "based on" line read from `resolvedPanelsFor(seg, presets)` (`../timeline/layoutTrack.ts`) - the same resolution the preview uses - rather than from `seg.arrangement` alone, so a segment with no arrangement still reports its preset's real visibility correctly.
- Every arrangement write is built by `setArrangementOp` (`../stage/arrange/arrangeMath.md`), shared with the stage drag, so the "first write bases off both-hidden" rule has exactly one implementation.
