# src/lib/edit.ts

TypeScript mirror of the Rust `edit::model` types that represent a recording session's editable state. These types cross the Tauri IPC boundary serialized as JSON with snake_case field names matching the Rust serde layout. Keep this file in sync with `src-tauri/src/edit/model.rs`.

## ZoomTarget

```ts
export type ZoomTarget = "cursor" | { fixed: { x: number; y: number } };
```

Discriminated union describing what a zoom region focuses on.

- `"cursor"` - the zoom follows the live cursor position frame-by-frame during the zoom span. *Why a string literal rather than an object variant:* matches the Rust serde untagged representation for the unit variant; a plain string is cheaper to serialize than `{ cursor: {} }`.
- `{ fixed: { x: number; y: number } }` - the zoom is anchored to a static point in frame coordinates. *Why frame coordinates rather than screen coordinates:* the exporter works in frame space; storing screen coordinates would require a conversion at render time and would break if the capture resolution changes.

### Used by

- `src/lib/edit.ts` - field `Zoom.target`.
- `src/lib/ipc.ts` - appears in `EditOp` variant `update_zoom`'s `target?` field.

## Zoom

```ts
export interface Zoom {
  id: string;
  start_ms: number;
  end_ms: number;
  target: ZoomTarget;
  scale: number;
  easing: string;
  zoom_in_ms: number;
  zoom_out_ms: number;
  layer: number;
  cam_action?: CamZoomAction | null;
}
```

One zoom segment within a recording.

- `id: string` - stable UUID assigned by the backend when the zoom is created. *Why needed:* `update_zoom` and `remove_zoom` ops reference zooms by id rather than by index to survive reordering.
- `start_ms: number` - playback time (milliseconds from start) when the zoom begins. *Why ms:* all timeline positions across `EditDoc` use milliseconds for a uniform unit.
- `end_ms: number` - playback time when the zoom ends (ease-out completes). The exporter interpolates the camera within `[start_ms, end_ms]`.
- `target: ZoomTarget` - what the zoom focuses on. *Why stored per-zoom:* different zooms within the same recording may need to follow the cursor or pin to a specific UI element.
- `scale: number` - zoom multiplier (e.g., `2.0` = 2x zoom). *Why a raw float:* matches the Rust `f64` field; lets the exporter multiply the frame transform matrix directly.
- `easing: string` - easing function name (e.g., `"ease_in_out"`) passed verbatim to the renderer. *Why a string rather than an enum:* allows new easing names to be added on the Rust side without a TS schema change.
- `zoom_in_ms` / `zoom_out_ms: number` - independent ease-in and ease-out durations within the zoom's span, edited via `ZoomInspector`'s "Zoom in"/"Zoom out" fields (and its Subtle/Balanced/Punchy presets). *Why split from a single duration:* a punchy zoom-in paired with a slower zoom-out (or vice versa) is a distinct, common feel that one symmetric duration can't express.
- `layer: number` - stacking index among zooms. Assigned by `layoutRegions` (`src/editor/timeline/layers.ts`) for timeline row placement, and read by `resolveCamAction` (`src/editor/stage/camZoomAction.ts`) to pick which of several time-overlapping zooms' `cam_action` governs the webcam - the highest layer wins, ties going to the last one, mirroring the Rust `max_by_key`.
- `cam_action?: CamZoomAction | null` - per-zoom webcam override; absent or `null` inherits the global `ZoomSettings.cam_zoom_default`. Set via the `set_zoom_cam_action` op.

### Used by

- `src/lib/edit.ts` - field `EditDoc.zooms`.
- `src/lib/ipc.ts` - `getEdit` and `applyEditOp` return `EditDoc` which contains `zooms: Zoom[]`.
- `src/editor/inspectors/ZoomInspector.tsx` - edits `scale`, `start_ms`/`end_ms`, `zoom_in_ms`/`zoom_out_ms`, `easing`, and `target` via `update_zoom` patches.
- `src/editor/stage/camZoomAction.ts` - `resolveCamAction` reads `layer` and `cam_action` to resolve the webcam's behavior during overlapping zooms.

## Cut

```ts
export interface Cut { start_ms: number; end_ms: number }
```

A time range to remove from the exported video.

- `start_ms: number` - start of the cut in playback milliseconds. *Why no id:* cuts are applied as a set by the exporter; they are not referenced individually by ops (use `add_cut` to append, `set_trim` to adjust the in/out points).
- `end_ms: number` - end of the cut; frames in `[start_ms, end_ms)` are dropped from the export.

### Used by

- `src/lib/edit.ts` - field `EditDoc.cuts`.
- `src/lib/ipc.ts` - present in `EditDoc` returned by `getEdit`, `applyEditOp`, and `aiAutoedit`.

## Speed

```ts
export interface Speed { id: string; start_ms: number; end_ms: number; factor: number }
```

A speed-ramp segment that stretches or compresses a time range in the export.

- `id: string` - stable identifier assigned by the backend. *Why an id:* future UI may allow editing or removing individual speed segments.
- `start_ms: number` - start of the ramp in playback milliseconds.
- `end_ms: number` - end of the ramp.
- `factor: number` - speed multiplier; `2.0` = 2x speed (shorter in export), `0.5` = half speed (longer in export). *Why a raw float:* direct multiplier for the encoder's frame timing math.

### Used by

- `src/lib/edit.ts` - field `EditDoc.speed`.
- `src/lib/ipc.ts` - present in `EditDoc`.

## LayoutSeg

```ts
export interface LayoutSeg { id: string; start_ms: number; end_ms: number; layout: string; transition_ms: number; easing: string; transition_out_ms: number; easing_out: string }
```

A time range in which a specific output layout (screen-only, picture-in-picture, presenter, etc.) is active.

- `id: string` - stable identifier.
- `start_ms / end_ms: number` - playback time range for this segment.
- `layout: string` - layout mode name (e.g., `"screen"`, `"presenter"`, `"camera_only"`). *Why a string rather than a typed union:* matches the Rust serde representation and allows new layout names to be added without regenerating the TS types.
- `transition_ms: number` - cross-fade duration (ms) blending IN from whatever layout preceded this segment; the blend STARTS at `start_ms`.
- `transition_out_ms: number` / `easing_out: string` - the exit cross-fade, which COMPLETES at `end_ms`. `0` is a hard cut. *Why these are required here despite being serde-defaulted in Rust:* Rust always SERIALIZES them, so every doc that reaches TypeScript has them - only files on disk can be missing them.
- `easing: string` - easing function name for that cross-fade, same free-form string convention as `Zoom.easing`.

### Used by

- `src/lib/edit.ts` - field `EditDoc.layout`.
- `src/lib/ipc.ts` - present in `EditDoc`; the `add_layout_seg` / `update_layout_seg` / `remove_layout_seg` `EditOp` variants add, patch, and delete segments by id.
- `src/editor/inspectors/LayoutInspector.tsx` - edits a segment's fields (including `transition_ms`/`easing`) via `update_layout_seg`, and deletes it via `remove_layout_seg`.
- `src/editor/timeline/layoutTrack.ts` - `layoutAt` cross-fades between segments' resolved panel rects for the preview, honoring each segment's own `transition_ms`/`easing`.

## Trim

```ts
export interface Trim { in_ms: number; out_ms: number }
```

In/out trim points for the recording. Frames before `in_ms` and after `out_ms` are excluded from the export.

- `in_ms: number` - trim-in point in playback milliseconds. *Why ms not frame index:* consistent with all other timeline units; the exporter converts to frames internally.
- `out_ms: number` - trim-out point. A value equal to the recording duration means no trim at the end.

### Used by

- `src/lib/edit.ts` - field `EditDoc.trim`.
- `src/lib/ipc.ts` - `set_trim` `EditOp` variant sets both fields atomically.

## Aspect

```ts
export type Aspect = "source" | "wide_16x9" | "vertical_9x16" | "square_1x1" | "classic_4x3";
```

Output frame aspect ratio - mirrors Rust `export::types::Aspect`. `"source"` (the default) matches today's behavior (the frame adapts to the recording's own dimensions); the 4 fixed presets pin a base resolution at that ratio. Never crops: the screen aspect-fits inside the (possibly resized) frame and the background fills the rest.

### Used by

- `src/lib/edit.ts` - field `EditDoc.aspect`; `EditOp` variant `set_aspect`.
- `src/editor/stage/Transport.tsx` - the aspect chip cycles through all 5 values and calls `set_aspect`.

## resolveTrim

```ts
export function resolveTrim(trim: Trim, durMs: number): { inMs: number; outMs: number }
```

Effective trim range against the clip's real duration - mirrors Rust `Trim::resolve` exactly, so the preview (playhead clamp, timeline dimming) always agrees with what export will cut. `trim.out_ms === 0` (unset) reads as "no trim yet": the whole clip.

### Inputs

- `trim: Trim` - the doc's raw trim fields.
- `durMs: number` - the clip's real duration (ms).

### Returns

`{ inMs, outMs }` - both clamped into `[0, durMs]`, with `inMs` never exceeding `outMs`.

### Used by

- `src/editor/Editor.tsx` - clamps playback to the trim range and snaps the play-start to `inMs`.
- `src/editor/timeline/TrimOverlay.tsx` - the dimmed head/tail regions and drag-handle positions.
- `src/editor/hooks/useTrimActions.ts` - resolves the current range once so `onTrimIn`/`onTrimOut` can reject a playhead click that would collapse or invert the trim.

## EditDoc

```ts
export interface EditDoc {
  version: number;
  trim: Trim;
  cuts: Cut[];
  zooms: Zoom[];
  speed: Speed[];
  layout: LayoutSeg[];
  effects: EffectRegion[];
  camera_moves: CameraMove[];
  aspect: Aspect;
  settings: Settings;
}
```

The complete editable state for one recording session. Loaded from `edit.json` in the project folder by `getEdit`, mutated via `applyEditOp`, and rendered by the Rust exporter.

- `version: number` - schema version number. *Why:* allows the backend to detect and migrate older `edit.json` files before the exporter reads them.
- `trim: Trim` - global in/out trim applied before any other edit.
- `cuts: Cut[]` - time ranges removed from the export; applied after trim.
- `zooms: Zoom[]` - zoom segments, ordered ascending by `start_ms`. *Why ordered:* the exporter walks them in order; out-of-order entries would produce incorrect frame transforms.
- `speed: Speed[]` - speed-ramp segments.
- `layout: LayoutSeg[]` - output-layout segments; typically one segment covering the full recording, overridden for specific time ranges.
- `effects: EffectRegion[]` - editable effect regions (v1: spotlight); empty is a no-op. *Why separate from `layout`:* effects overlay on top of whichever layout is active rather than replacing it, and can overlap each other (stacked via each region's `layer`).
- `camera_moves: CameraMove[]` - webcam PiP keyframes; empty is a no-op (byte-identical to before the feature existed).
- `aspect: Aspect` - output frame aspect ratio; `"source"` (the default) matches today's behavior exactly, so a doc saved before this field existed loads unchanged.
- `settings: Settings` - snapshot of the recorder settings at the time of capture. *Why embedded:* the exporter is fully self-contained per project; it must not depend on the current live settings, which may have changed since recording.

### Used by

- `src/lib/ipc.ts` - return type of `getEdit`, `applyEditOp`, `aiAutoedit`; parameter type of `saveEdit`.

## EditOp

```ts
export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | { op: "update_zoom"; id: string; start_ms?: number; end_ms?: number; scale?: number; target?: ZoomTarget; easing?: string; zoom_in_ms?: number; zoom_out_ms?: number; layer?: number }
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
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | { op: "update_effect"; id: string; start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number; layer?: number }
  | { op: "remove_effect"; id: string }
  | { op: "add_camera_move"; t_ms: number; x: number; y: number; size: number }
  | { op: "update_camera_move"; id: string; t_ms?: number; x?: number; y?: number; size?: number; easing?: string }
  | { op: "remove_camera_move"; id: string };
```

Discriminated union of all edit verbs. Each variant is tagged by the `op` string literal field, matching the Rust `EditOp` enum's serde tag. The AI director and the GUI editor both emit these.

- `add_zoom` - appends a new zoom at `at_ms` with default scale and duration `dur_ms`. *Why a separate variant from `add_zoom_full`:* lets the backend apply sensible defaults (scale from `ZoomConfig`) without the caller specifying them.
- `add_zoom_full` - appends a zoom with an explicit `scale`. *Why:* the AI director knows what scale to use and must not be overridden by the default.
- `update_zoom` - patches any subset of a zoom's fields by `id`: timing, `scale`, `target`, `easing`, the asymmetric `zoom_in_ms`/`zoom_out_ms` ease durations, and `layer`. All fields except `id` are optional. *Why partial update:* a drag-to-resize gesture changes only `end_ms`; sending the full zoom would be verbose and race-prone.
- `remove_zoom` - deletes the zoom with the given `id`.
- `clear_zooms` - deletes every zoom in the doc. *Why:* the AI director's agentic plan (`aiPlan`) leads with this step (labeled "Rethinking your zooms…") whenever the doc already has zooms, so the reveal shows the mechanical seed-time zooms give way cleanly to the smart ones instead of overlaying on top of them.
- `set_zoom_cam_action` - sets (or clears, with `null`) a single zoom's `cam_action` override, superseding the global `ZoomSettings.cam_zoom_default` for just that zoom's span.
- `set_trim` - replaces the document's `trim` in/out points atomically. *Why both together:* trim is always a pair; a partial update would leave an inconsistent state.
- `set_aspect` - replaces `EditDoc.aspect`; the next export or preview build re-resolves the output `Layout` from it.
- `add_cut` - appends a new `Cut` for the given time range.
- `set_speed` - sets or replaces the speed ramp covering `[start_ms, end_ms]` with the given `factor`. *Why set rather than add:* the backend merges or replaces overlapping speed segments; the caller describes the desired outcome, not the mutation.
- `add_layout_seg` - appends a new `LayoutSeg` of `layout` starting at `at_ms` with duration `dur_ms`. `transition_out_ms`/`easing_out` are optional; omitted, the segment gets the hard-cut default, so existing callers are unchanged.
- `update_layout_seg` - patches any subset of a layout segment's fields by `id`: time range, `layout` name, `transition_ms`, `easing`, `transition_out_ms`, `easing_out`. *Why partial update:* dragging a segment's edge on the timeline changes only `start_ms`/`end_ms`.
- `remove_layout_seg` - deletes the layout segment with the given `id`.
- `add_effect` - appends a new `EffectRegion` of `kind` for the given time range.
- `update_effect` - patches any subset of an effect region's fields by `id`, including `layer` for overlap stacking.
- `remove_effect` - deletes the effect region with the given `id`.
- `add_camera_move` - appends a new webcam PiP keyframe (`CameraMove`) at `t_ms` with position `x`/`y` and `size`.
- `update_camera_move` - patches any subset of a camera-move keyframe's fields by `id`: time, position, size, `easing`.
- `remove_camera_move` - deletes the camera-move keyframe with the given `id`.

### Used by

- `src/lib/ipc.ts` - parameter type of `applyEditOp`; also the type of `AiStep.op`, since the AI director's plan is just a labeled sequence of these.

## EffectKind

```ts
export type EffectKind = "spotlight";
```

The kind of an editable effect region (mirrors the Rust `EffectKind`). The set grows over phases.

## EffectRegion

```ts
export interface EffectRegion { id: string; kind: EffectKind; start_ms: number; end_ms: number; fade_in_ms: number; fade_out_ms: number; mode?: string; dim?: number; radius?: number; feather?: number; layer: number }
```

An editable effect region on the timeline (v1: spotlight), held in `EditDoc.effects`. The `add_effect` / `update_effect` / `remove_effect` `EditOp`s mutate it; the timeline lays them into layers (`layoutRegions`) and draws each as a draggable pill. `layer: number` doubles as that timeline row-stacking index and, for regions overlapping at the same instant, an explicit priority - the preview's `spotlightPreview.ts` picks the highest-`layer` region active at a given time (ties go to the last one in the array), mirroring the Rust `max_by_key` overlap rule.

## CameraMove

```ts
export interface CameraMove { id: string; t_ms: number; x: number; y: number; size: number; easing: string }
```

One webcam PiP keyframe on the Camera lane - a "Move mode" drag-and-drop point the export interpolates between (mirrors the Rust `CameraMove`). An empty `camera_moves` array is a no-op, byte-identical to before the feature existed.

- `id: string` - stable identifier; `update_camera_move`/`remove_camera_move` reference a keyframe by id.
- `t_ms: number` - playback time of this keyframe.
- `x` / `y: number` - the webcam panel's center position, 0..1 fraction of the output frame (same basis as `ZoomTarget`'s fixed point).
- `size: number` - the webcam panel's size at this keyframe, 0..1 fraction of the output's short edge (same basis as `ModeAppearance.cam_size`).
- `easing: string` - easing function name for the interpolation into this keyframe, same free-form string convention as `Zoom.easing`.

### Used by

- `src/lib/edit.ts` - field `EditDoc.camera_moves`.
- `src/editor/stage/cameraMoves.ts` - samples/interpolates between keyframes (`CamPose`) for the live preview.
- `src/editor/inspectors/CameraMoveInspector.tsx` - the inspector shown when a keyframe is selected; every field maps to an `update_camera_move` patch.
- `src/editor/timeline/CameraLane.tsx` - renders and drags keyframes on the timeline.
