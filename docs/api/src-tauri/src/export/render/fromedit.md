# src-tauri/src/export/render/fromedit.rs

Since the time remap the doc this reads has already been through `edit::remap_doc` (in `render_edit::EditState::load`): trim, cuts and speed are consumed and every span below is on the output clock.

Inverse of `edit::seed`: reconstructs the exporter's raw `ZoomRegion` list and `SetLayout` action track from a persisted `EditDoc`, so the exporter renders the user's saved plan instead of regenerating it from click events. Trim, cuts, and speed changes are intentionally not applied here. The round-trip proof test `regions_round_trip_through_edit_doc` demonstrates lossless `seed <-> export` conversion.

## regions_from_doc

```rust
pub fn regions_from_doc(doc: &EditDoc, sw: u32, sh: u32) -> Vec<ZoomRegion>
```

Pure inverse of `seed::zooms_from_regions`. Rebuilds one `ZoomRegion` per `Zoom` in the doc, in source order.

### Inputs

- `doc: &EditDoc` - the persisted edit document containing the user's zoom timeline and settings snapshot. *Why:* all zoom data lives here after the user edits it in the editor; the exporter reads this instead of re-running autozoom.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* only consulted for the `ZoomTarget::Cursor` fallback; `Fixed` anchors (every seeded zoom) are reproduced exactly without these values.* Since a `Cursor` zoom also sets `follow_cursor`, `CameraSim` aims at the live cursor and never reads that fallback - it only keeps the field total.

### Returns

`Vec<ZoomRegion>` - one region per `doc.zooms` entry. Empty when `doc.zooms` is empty.

### Implementation

1. Call `doc.settings.zoom.to_zoom_config()` to recover `zoom_in_ms`, `zoom_out_ms`, and `easing`. *Why re-derive from settings:* these fields are not stored per-`Zoom` (they are uniform across the doc); the settings snapshot in the doc reproduces the same config that was active when the doc was seeded.*
2. For each `Zoom`, call `anchor_for(z, sw, sh)` to recover the `FramePoint` and `easing_from(&z.easing, cfg.easing)` to recover the `Easing` variant.
3. Build `ZoomRegion { start_ms, end_ms, zoom_in_ms, zoom_out_ms, target_scale: z.scale, anchor, easing, layer, cam_action, follow_cursor: matches!(z.target, ZoomTarget::Cursor) }`. *Why the flag rather than resolving a point here:* there is no cursor track at this layer, and even with one a point resolved at `start_ms` goes stale the moment the pill is dragged along the timeline - `CameraSim` re-reads the live cursor every step instead.

### Behaviors worth knowing

- `regions_round_trip_through_edit_doc` - regions -> `seed::zooms_from_regions` -> `EditDoc` -> `regions_from_doc` returns field-identical regions. This is the proof of losslessness.
- `empty_zooms_make_no_regions` - default `EditDoc` produces an empty vec.
- `cursor_target_defaults_to_screen_center` - `ZoomTarget::Cursor` anchor becomes `(sw/2, sh/2)` (and `follow_cursor` becomes `true`, which is what makes that value inert).
- See `easing_from` below for how the `easing` field is reconstructed, including the "spring" fix.

## easing_from

```rust
pub fn easing_from(name: &str, cfg_easing: Easing) -> Easing
```

Maps an easing wire-name (from a `Zoom.easing` or `LayoutSeg.easing`/`easing_out` string) back to `Easing`. Named curves map directly; the two PARAMETERISED forms - `spring(stiffness,damping[,mass])` and `cubic(x1,y1,x2,y2)` - reconstruct their whole shape via `export::spring::parse_spring` / `export::cubic::parse_cubic`; anything else falls back to `cfg_easing`.

### Inputs

- `name: &str` - the wire-name easing string.
- `cfg_easing: Easing` - the fallback for a name this function cannot reconstruct. Every caller today passes `Easing::Smooth` (a seeded doc's config, or `LayoutTrack::from_segs`'s literal `Easing::Smooth`).

### Returns

`Easing` - the reconstructed curve, or `cfg_easing` when `name` is neither a known curve nor a parseable spring or cubic.

### Implementation

1. `"smooth"` / `"linear"` / `"spring"` / `"ease_in"` / `"ease_out"` / `"ease_in_out"` map directly to their `Easing` variant. The bare `"spring"` carries no parameters, so it maps to `SPRING_DEFAULT` (`export/types.md`).
2. Anything else tries `export::spring::parse_spring` FIRST, then `export::cubic::parse_cubic`; a match reconstructs `Easing::Spring` / `Easing::Cubic` exactly, mass defaulting to 1. *Why spring first:* the two prefixes are disjoint, so the order is only about cost, and a failed `parse_spring` is a cheap prefix miss.
3. Otherwise returns `cfg_easing`.

### Behaviors worth knowing

- **Preview/export divergence bug (fixed):** `"spring"` used to have no arm, fall into the `_` branch, fail `parse_cubic`, and silently return `cfg_easing` (`Easing::Smooth` in practice) - so the "Punchy" zoom preset or the CurveEditor's Spring card overshot in the live preview but rendered as a plain Smooth ease in the export. `spring_maps_to_a_real_spring_variant` pins the mapping, including that `spring(300,10)` and `spring(300,10,2)` reconstruct exactly and `spring(300)` (wrong arity) falls back.
- The spring tests live in the sibling `fromedit_spring_tests.rs` (split out so both files stay under the size limit): `spring_ease_matches_the_ts_mirror_and_overshoots` pins numeric parity with `src/lib/spring.test.ts` through `camera::ease` AND that overshoot tracks the damping ratio; `spring_layout_transition_overshoots_the_destination_scene` proves it end-to-end through `LayoutTrack::from_segs`, and that `spring(300,10)` (zeta 0.289) extrapolates further past the destination scene than `spring(170,26)` (zeta 0.997), which barely passes it.
- `easing_unknown_falls_back_to_config` - an unrecognized string falls back to `cfg_easing`; "smooth"/"linear" reconstruct exactly.
- A `cubic(x1,y1,x2,y2)` string is also reconstructed exactly, into `Easing::Cubic` - it is the fallback arm's FIRST try, so only a genuinely unparseable name reaches `cfg_easing`. That is what makes a custom curve survive the doc round trip: the string carries the whole shape - and since `spring(...)` landed, so does a spring's stiffness/damping/mass.

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
