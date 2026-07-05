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
}
```

One zoom segment within a recording.

- `id: string` - stable UUID assigned by the backend when the zoom is created. *Why needed:* `update_zoom` and `remove_zoom` ops reference zooms by id rather than by index to survive reordering.
- `start_ms: number` - playback time (milliseconds from start) when the zoom begins. *Why ms:* all timeline positions across `EditDoc` use milliseconds for a uniform unit.
- `end_ms: number` - playback time when the zoom ends (ease-out completes). The exporter interpolates the camera within `[start_ms, end_ms]`.
- `target: ZoomTarget` - what the zoom focuses on. *Why stored per-zoom:* different zooms within the same recording may need to follow the cursor or pin to a specific UI element.
- `scale: number` - zoom multiplier (e.g., `2.0` = 2x zoom). *Why a raw float:* matches the Rust `f64` field; lets the exporter multiply the frame transform matrix directly.
- `easing: string` - easing function name (e.g., `"ease_in_out"`) passed verbatim to the renderer. *Why a string rather than an enum:* allows new easing names to be added on the Rust side without a TS schema change.

### Used by

- `src/lib/edit.ts` - field `EditDoc.zooms`.
- `src/lib/ipc.ts` - `getEdit` and `applyEditOp` return `EditDoc` which contains `zooms: Zoom[]`.

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
export interface LayoutSeg { id: string; start_ms: number; end_ms: number; layout: string }
```

A time range in which a specific output layout (screen-only, picture-in-picture, presenter, etc.) is active.

- `id: string` - stable identifier.
- `start_ms / end_ms: number` - playback time range for this segment.
- `layout: string` - layout mode name (e.g., `"screen"`, `"presenter"`, `"camera_only"`). *Why a string rather than a typed union:* matches the Rust serde representation and allows new layout names to be added without regenerating the TS types.

### Used by

- `src/lib/edit.ts` - field `EditDoc.layout`.
- `src/lib/ipc.ts` - present in `EditDoc`; the `set_layout_seg` `EditOp` variant updates a segment's `layout` field by id.

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

## EditDoc

```ts
export interface EditDoc {
  version: number;
  trim: Trim;
  cuts: Cut[];
  zooms: Zoom[];
  speed: Speed[];
  layout: LayoutSeg[];
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
- `settings: Settings` - snapshot of the recorder settings at the time of capture. *Why embedded:* the exporter is fully self-contained per project; it must not depend on the current live settings, which may have changed since recording.

### Used by

- `src/lib/ipc.ts` - return type of `getEdit`, `applyEditOp`, `aiAutoedit`; parameter type of `saveEdit`.

## EditOp

```ts
export type EditOp =
  | { op: "add_zoom"; at_ms: number; dur_ms: number }
  | { op: "add_zoom_full"; at_ms: number; dur_ms: number; scale: number }
  | { op: "update_zoom"; id: string; start_ms?: number; end_ms?: number; scale?: number; target?: ZoomTarget; easing?: string; zoom_in_ms?: number; zoom_out_ms?: number }
  | { op: "remove_zoom"; id: string }
  | { op: "set_trim"; in_ms: number; out_ms: number }
  | { op: "add_cut"; start_ms: number; end_ms: number }
  | { op: "set_speed"; start_ms: number; end_ms: number; factor: number }
  | { op: "set_layout_seg"; id: string; layout: string }
  | { op: "add_effect"; kind: EffectKind; start_ms: number; end_ms: number }
  | { op: "update_effect"; id: string; start_ms?: number; end_ms?: number; fade_in_ms?: number; fade_out_ms?: number; mode?: string; dim?: number; radius?: number; feather?: number }
  | { op: "remove_effect"; id: string }
```

Discriminated union of all edit verbs. Each variant is tagged by the `op` string literal field, matching the Rust `EditOp` enum's serde tag. The AI director and the GUI editor both emit these.

- `add_zoom` - appends a new zoom at `at_ms` with default scale and duration `dur_ms`. *Why a separate variant from `add_zoom_full`:* lets the backend apply sensible defaults (scale from `ZoomConfig`) without the caller specifying them.
- `add_zoom_full` - appends a zoom with an explicit `scale`. *Why:* the AI director knows what scale to use and must not be overridden by the default.
- `update_zoom` - patches any subset of a zoom's fields by `id`. All fields except `id` are optional. *Why partial update:* a drag-to-resize gesture changes only `end_ms`; sending the full zoom would be verbose and race-prone.
- `remove_zoom` - deletes the zoom with the given `id`.
- `set_trim` - replaces the document's `trim` in/out points atomically. *Why both together:* trim is always a pair; a partial update would leave an inconsistent state.
- `add_cut` - appends a new `Cut` for the given time range.
- `set_speed` - sets or replaces the speed ramp covering `[start_ms, end_ms]` with the given `factor`. *Why set rather than add:* the backend merges or replaces overlapping speed segments; the caller describes the desired outcome, not the mutation.
- `set_layout_seg` - changes the `layout` field of the segment identified by `id`.

### Used by

- `src/lib/ipc.ts` - parameter type of `applyEditOp`.

## EffectKind

```ts
export type EffectKind = "spotlight";
```

The kind of an editable effect region (mirrors the Rust `EffectKind`). The set grows over phases.

## EffectRegion

```ts
export interface EffectRegion { id: string; kind: EffectKind; start_ms: number; end_ms: number; fade_in_ms: number; fade_out_ms: number; mode?: string; dim?: number; radius?: number; feather?: number }
```

An editable effect region on the timeline (v1: spotlight), held in `EditDoc.effects`. The `add_effect` / `update_effect` / `remove_effect` `EditOp`s mutate it; the timeline lays them into layers (`layoutRegions`) and draws each as a draggable pill.
