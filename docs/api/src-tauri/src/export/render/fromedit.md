# src-tauri/src/export/render/fromedit.rs

Inverse of `edit::seed`: reconstructs the exporter's raw `ZoomRegion` list and `SetLayout` action track from a persisted `EditDoc`, so the exporter renders the user's saved plan instead of regenerating it from click events. Trim, cuts, and speed changes are intentionally not applied here. The round-trip proof test `regions_round_trip_through_edit_doc` demonstrates lossless `seed <-> export` conversion.

## regions_from_doc

```rust
pub fn regions_from_doc(doc: &EditDoc, sw: u32, sh: u32) -> Vec<ZoomRegion>
```

Pure inverse of `seed::zooms_from_regions`. Rebuilds one `ZoomRegion` per `Zoom` in the doc, in source order.

### Inputs

- `doc: &EditDoc` - the persisted edit document containing the user's zoom timeline and settings snapshot. *Why:* all zoom data lives here after the user edits it in the editor; the exporter reads this instead of re-running autozoom.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* only consulted for the `ZoomTarget::Cursor` fallback; `Fixed` anchors (every seeded zoom) are reproduced exactly without these values.*

### Returns

`Vec<ZoomRegion>` - one region per `doc.zooms` entry. Empty when `doc.zooms` is empty.

### Implementation

1. Call `doc.settings.zoom.to_zoom_config()` to recover `zoom_in_ms`, `zoom_out_ms`, and `easing`. *Why re-derive from settings:* these fields are not stored per-`Zoom` (they are uniform across the doc); the settings snapshot in the doc reproduces the same config that was active when the doc was seeded.*
2. For each `Zoom`, call `anchor_for(z, sw, sh)` to recover the `FramePoint` and `easing_from(&z.easing, cfg.easing)` to recover the `Easing` variant.
3. Build `ZoomRegion { start_ms, end_ms, zoom_in_ms, zoom_out_ms, target_scale: z.scale, anchor, easing }`.

### Behaviors worth knowing

- `regions_round_trip_through_edit_doc` - regions -> `seed::zooms_from_regions` -> `EditDoc` -> `regions_from_doc` returns field-identical regions. This is the proof of losslessness.
- `empty_zooms_make_no_regions` - default `EditDoc` produces an empty vec.
- `cursor_target_defaults_to_screen_center` - `ZoomTarget::Cursor` anchor becomes `(sw/2, sh/2)`.
- `easing_unknown_or_spring_falls_back_to_config` - "spring" and unknown easing strings fall back to the config's easing; "smooth", "linear", "ease_in", "ease_out", and "ease_in_out" are reconstructed exactly.

## layout_segs_from_doc

```rust
pub fn layout_segs_from_doc(doc: &EditDoc) -> Option<Vec<ActionEvent>>
```

Pure inverse of `seed::layout_from_actions`. Rebuilds the `SetLayout` action track that `LayoutTrack::new` consumes.

### Inputs

- `doc: &EditDoc` - the persisted edit document. *Why:* the layout timeline is stored as `doc.layout` (a `Vec<LayoutSeg>`) after seeding.*

### Returns

`Option<Vec<ActionEvent>>` - `None` when `doc.layout` is empty, which is the caller's signal to fall back to the raw `actions.json` track. `Some` with one `SetLayout` action per segment otherwise.

### Implementation

1. Return `None` if `doc.layout.is_empty()`.
2. Map each `LayoutSeg s` to `ActionEvent { t: s.start_ms, kind: ActionKind::SetLayout(layout_id_from(&s.layout)) }`. Layout name strings are deserialized via serde JSON to `LayoutId`; unknown names fall back to `LayoutId::Screen`.

### Behaviors worth knowing

- `empty_layout_signals_fallback` - default `EditDoc` -> `None`.
- `layout_segs_round_trip` - action track -> `seed::layout_from_actions` -> `EditDoc` -> `layout_segs_from_doc` rebuilds a track whose `SetLayout` switches match the input. The seed prepends a `(0, Screen)` base segment; the rebuilt track emits a corresponding `SetLayout(Screen)@0` that is a benign duplicate because `LayoutTrack::new` injects its own `(0, Screen)` base.
