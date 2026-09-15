# src/shared/editOps.ts

The edit verb union, split out of `edit.ts`: that file is the document shape, this one is every mutation you can ask the backend to make to it. `edit.ts` re-exports `EditOp`, so `import type { EditOp } from "../shared/edit"` keeps working everywhere. The types the variants are built from (`ZoomTarget`, `PanelPose`, `Aspect`, `EffectKind`, `CamMoveShape`, `Caption`) stay in `edit.ts` and are imported back here - a type-only cycle, erased at build time.

## EditOp

Time remap ops (mirroring `edit::ops::timeops`): `add_cuts { spans }` (one undo step), `update_cut`, `remove_cut`, `update_speed`, `remove_speed`, beside the existing `add_cut` and `set_speed`. `Cut` carries an `id` (`c{n}`).

```ts
export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | { op: "update_zoom"; id: string; start_ms?: number; end_ms?: number; scale?: number; target?: ZoomTarget; easing?: string; easing_out?: string; zoom_in_ms?: number; zoom_out_ms?: number; layer?: number; smart_typing?: boolean }
  | { op: "remove_zoom"; id: string }
  | { op: "clear_zooms" }
  | { op: "set_zoom_cam_action"; id: string; action: CamZoomAction | null }
  | { op: "set_trim"; in_ms: number; out_ms: number }
  | { op: "set_aspect"; aspect: Aspect }
  | { op: "add_cut"; start_ms: number; end_ms: number }
  | { op: "set_speed"; start_ms: number; end_ms: number; factor: number }
  | { op: "add_layout_seg"; at_ms: number; dur_ms: number; layout: string }
  | { op: "update_layout_seg"; id: string; start_ms?: number; end_ms?: number; layout?: string; transition_ms?: number; easing?: string }
  | { op: "remove_layout_seg"; id: string }
  | { op: "set_arrangement"; id: string; screen?: PanelPose | null; cam?: PanelPose | null }
  | { op: "clear_arrangement"; id: string }
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | { op: "update_effect"; id: string; start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number; layer?: number }
  | { op: "remove_effect"; id: string }
  | { op: "add_camera_move"; t_ms: number; x: number; y: number; size: number; shape?: CamMoveShape; roundness?: number }
  | { op: "update_camera_move"; id: string; t_ms?: number; x?: number; y?: number; size?: number; easing?: string; shape?: CamMoveShape; roundness?: number }
  | { op: "remove_camera_move"; id: string }
  | { op: "apply_motion_default" }
  | { op: "update_caption"; id: string; start_ms?: number; end_ms?: number; text?: string }
  | { op: "remove_caption"; id: string }
  | { op: "merge_captions"; id: string }
  | { op: "split_caption"; id: string; at_ms: number }
  | { op: "set_captions"; captions: Caption[] }
  | { op: "clear_captions" };
```

Discriminated union of all edit verbs. Each variant is tagged by the `op` string literal field, matching the Rust `EditOp` enum's serde tag. The AI director and the GUI editor both emit these.

- `add_zoom` - appends a new zoom at `at_ms` with default scale and duration `dur_ms`. *Why a separate variant from `add_zoom_full`:* lets the backend apply sensible defaults (scale from `ZoomConfig`) without the caller specifying them.
- `add_zoom_full` - appends a zoom with an explicit `scale`. *Why:* the AI director knows what scale to use and must not be overridden by the default.
- `update_zoom` - patches any subset of a zoom's fields by `id`: timing, `scale`, `target`, `easing`, `easing_out`, the asymmetric `zoom_in_ms`/`zoom_out_ms` ease durations, `layer` and `smart_typing`. All fields except `id` are optional. **`easing_out` is THREE-valued through one optional string:** omit the key to leave the zoom's out ramp alone, send a curve to set it, send `""` to clear it back to "the same curve as `easing`". *Why a sentinel and not `null`:* the empty string is never a legal curve (Rust's `valid_easing` degrades it), so it is free to carry the clear, and the op stays a flat object that `JSON.stringify` cannot garble the way it drops `undefined`. Send `easing` and `easing_out` in the SAME op when both change: Rust applies `easing` first, and an out ramp equal to it collapses back to unset rather than storing a duplicate. *Why partial update:* a drag-to-resize gesture changes only `end_ms`; sending the full zoom would be verbose and race-prone.
- `remove_zoom` - deletes the zoom with the given `id`.
- `clear_zooms` - deletes every zoom in the doc. *Why:* a pass that replaces the seeded zooms wholesale can start clean instead of overlaying on top of them.
- `set_zoom_cam_action` - sets (or clears, with `null`) a single zoom's `cam_action` override, superseding the global `ZoomSettings.cam_zoom_default` for just that zoom's span.
- `set_trim` - replaces the document's `trim` in/out points atomically. *Why both together:* trim is always a pair; a partial update would leave an inconsistent state.
- `set_aspect` - replaces `EditDoc.aspect`; the next export or preview build re-resolves the output `Layout` from it.
- `add_cut` - appends a new `Cut` for the given time range.
- `set_speed` - sets or replaces the speed ramp covering `[start_ms, end_ms]` with the given `factor`. *Why set rather than add:* the backend merges or replaces overlapping speed segments; the caller describes the desired outcome, not the mutation.
- `add_layout_seg` - appends a new `LayoutSeg` of `layout` starting at `at_ms` with duration `dur_ms`. `transition_out_ms`/`easing_out` are optional; omitted, the segment gets the SAME cross-fade as its entry (`NEW_LAYOUT_TRANSITION_MS` = 350ms / `"smooth"`, see `api.md`), i.e. a new segment is symmetric. Pass `transition_out_ms: 0` to ask for an explicit hard cut.
- `update_layout_seg` - patches any subset of a layout segment's fields by `id`: time range, `layout` name, `transition_ms`, `easing`, `transition_out_ms`, `easing_out`. *Why partial update:* dragging a segment's edge on the timeline changes only `start_ms`/`end_ms`.
- `remove_layout_seg` - deletes the layout segment with the given `id`.
- `set_arrangement` - sets or hides a layout segment's panel poses. Each panel field is THREE-valued: **omit** the key to leave that panel as it is, pass `null` to hide it, pass a `PanelPose` to set (and un-hide) it. *Why omitted and `null` must differ:* they are the only way one op can patch two independent panels without a caller ever having to resend a pose it did not change. Note `JSON.stringify` drops `undefined` keys, so `{ cam: undefined }` correctly reads as "leave the cam alone" - but an explicit `cam: null` HIDES it, so never use `null` as a stand-in for "no value". On a segment with no arrangement yet the base is "both panels hidden", so converting a preset means sending BOTH panels: take them from the matching `LayoutPresetDto.arrangement` in `previewLayouts`. A change that would hide both panels is rejected by Rust (a no-op).
- `clear_arrangement` - drops a segment's arrangement so it resolves from its `layout` preset again; the way back from a custom arrangement.
- `add_effect` - appends a new `EffectRegion` of `kind` for the given time range.
- `update_effect` - patches any subset of an effect region's fields by `id`, including `layer` for overlap stacking.
- `remove_effect` - deletes the effect region with the given `id`.
- `add_camera_move` - sets the webcam PiP keyframe (`CameraMove`) at `t_ms` with position `x`/`y`, `size`, and optionally its `shape`/`roundness` (absent = `"layout"` / the backend default). An instant that already holds a keyframe is updated in place, not duplicated (Rust `EditOp::AddCameraMove`), so a slider committing twice before the doc round-trips cannot stack a stale twin.
- `update_camera_move` - patches any subset of a camera-move keyframe's fields by `id`: time, position, size, `easing`, `shape`, `roundness`.
- `remove_camera_move` - deletes the camera-move keyframe with the given `id`.
- `apply_motion_default` - stamps `settings.motion` onto every zoom, layout segment and camera move at once (Settings > Motion's "Apply to all regions"). One op, so the whole sweep is one undo step - the same reason `add_cuts` exists as a batch. Only the curves change; no timing, target or pose is touched. Wired from `Editor.tsx` through `EditorDialogs`' `onApplyMotion` prop.
- `update_caption` - patches any subset of a caption's `start_ms` / `end_ms` / `text` by `id`. The time fields clamp and re-order exactly like every other region. `text` keeps the caption's `words` ONLY when the new text is exactly those words joined by single spaces; any other edit clears them, because a word-by-word highlight driven by stale timings lights the wrong word (see `src/editor/inspectors/captionEdit.md` for the full argument). `CaptionTextField` commits once per edit rather than per keystroke, so this is one undo step per sentence.
- `remove_caption` - deletes the caption with the given `id`.
- `merge_captions` - joins this caption with the NEXT one in time order. The earlier caption's id survives, so the timeline selection does not jump. A no-op on the last caption, which is why `captionEdit::nextCaption` exists: the inspector disables Merge there instead of offering a press that does nothing.
- `split_caption` - cuts a caption in two at `at_ms`, which must be strictly INSIDE it (either edge is a no-op). Words go to the half they start in; a caption with no word timings splits its text at the nearest space instead. `CaptionInspector` only ever sends a word's own `start_ms` when the caption has word timings, so a split never lands mid-word.
- `set_captions` - replaces the whole track. What the ASR pass writes (`asr::commands::transcribe_project`, applied on the BACKEND under the doc lock); the frontend never sends it, and re-reads the doc with `getEdit` instead.
- `clear_captions` - empties the track. Behind a confirm in `CaptionsPanel`: transcribing again re-creates the lines, but hand-edited text is gone.

### Used by

- `src/shared/ipc.ts` - parameter type of `applyEditOp`.
