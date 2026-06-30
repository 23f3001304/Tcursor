# src-tauri/src/edit/model.rs

Data model for `edit.json`: the complete type hierarchy from atomic clip edits (`Trim`, `Cut`, `Zoom`, `Speed`, `LayoutSeg`) up to the root `EditDoc`, plus save/load helpers that make `EditDoc` self-serializing. All types derive `Serialize`/`Deserialize` so they transit IPC and disk without a separate DTO layer.

## Trim

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Trim { pub in_ms: u32, pub out_ms: u32 }
impl Default for Trim { fn default() -> Self { Self { in_ms: 0, out_ms: 0 } } }
```

The clip's in/out points in milliseconds, measured from the raw recording start.

- `in_ms` - *start of the exported region; frames before this are discarded. Default 0 = no head trim.*
- `out_ms` - *end of the exported region; frames after this are discarded. Also used as `duration_ms` in `metrics`. Default 0 signals "not yet set" and is replaced by `build_default` with the real clip duration.*

### Used by

- `src-tauri/src/edit/api.rs` - `SetTrim` variant replaces both fields; `metrics` reads `in_ms`/`out_ms`
- `src-tauri/src/export/fromedit.rs` - clip bounds supplied to the compositor

## Cut

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Cut { pub start_ms: u32, pub end_ms: u32 }
```

A single removed time range within the clip, clamped to the trim window at render time.

- `start_ms` - *inclusive start of the removed span.*
- `end_ms` - *exclusive end of the removed span.*

### Used by

- `src-tauri/src/edit/api.rs` - appended by `AddCut`; iterated in `metrics` to compute `kept_ms`
- `src-tauri/src/export/fromedit.rs` - passed to the compositor to skip frames in the cut range

## ZoomTarget

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomTarget { Cursor, Fixed { x: f32, y: f32 } }
```

Where the camera centers during a zoom event.

- `Cursor` - *track the live cursor position at each frame; appropriate for user-created zooms that should follow the pointer.*
- `Fixed { x, y }` - *anchor to a specific screen-local coordinate (floats); used by `seed.rs` to preserve the auto-zoom anchor from the recording's click position, keeping a re-render byte-identical to the original export.*

### Used by

- `src-tauri/src/edit/api.rs` - `AddZoom` defaults to `Cursor`; `AddZoomFull` also defaults to `Cursor`
- `src-tauri/src/edit/seed.rs` - `zooms_from_regions` always writes `Fixed` to lock the click anchor
- `src-tauri/src/export/fromedit.rs` - the compositor branches on this to compute the camera center per frame

## Zoom

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Zoom {
    pub id: String, pub start_ms: u32, pub end_ms: u32,
    pub target: ZoomTarget, pub scale: f32, pub easing: String,
}
```

One zoom event in the timeline.

- `id` - *stable string key (e.g. `"z0"`, `"z3"`) used by `UpdateZoom`/`RemoveZoom` to target specific entries without relying on list position.*
- `start_ms` - *frame time at which the zoom-in begins.*
- `end_ms` - *frame time at which the zoom-out completes.*
- `target` - *where the camera should point; see `ZoomTarget`.*
- `scale` - *peak zoom multiplier (e.g. `2.0` = 2x). Passed directly to the compositor.*
- `easing` - *named easing curve (`"smooth"`, `"linear"`, `"spring"`); looked up by the compositor at render time.*

### Used by

- `src-tauri/src/edit/api.rs` - inserted by `AddZoom`/`AddZoomFull`, mutated by `UpdateZoom`, removed by `RemoveZoom`
- `src-tauri/src/edit/seed.rs` - `zooms_from_regions` constructs the initial list
- `src-tauri/src/export/fromedit.rs` - rendered by `CameraSim` per frame
- `src-tauri/src/ai/plan.rs` - AI director reads and writes zoom entries

## Speed

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Speed { pub id: String, pub start_ms: u32, pub end_ms: u32, pub factor: f32 }
```

A playback-speed multiplier applied to a time range.

- `id` - *stable key (e.g. `"s0"`) for plan-based mutation.*
- `start_ms` / `end_ms` - *the span over which the speed change is active.*
- `factor` - *multiplier: `2.0` = double speed, `0.5` = half speed.*

### Used by

- `src-tauri/src/edit/api.rs` - appended by `SetSpeed`
- `src-tauri/src/export/fromedit.rs` - applied during frame-time remapping in the compositor

## LayoutSeg

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutSeg { pub id: String, pub start_ms: u32, pub end_ms: u32, pub layout: String }
```

A contiguous time span that uses a named screen layout (e.g. `"screen"`, `"pip"`, `"camera"`).

- `id` - *stable key (e.g. `"l0"`) used by `SetLayoutSeg` to target the segment to update.*
- `start_ms` / `end_ms` - *the span where this layout is active; segments must be non-overlapping and cover the full clip duration.*
- `layout` - *serde wire name of the layout variant (lowercase snake_case); consumed by the compositor to choose the frame composition template.*

### Used by

- `src-tauri/src/edit/api.rs` - `SetLayoutSeg` mutates `layout` on a matched entry
- `src-tauri/src/edit/seed.rs` - `layout_from_actions` builds the initial list from the `SetLayout` action track
- `src-tauri/src/export/fromedit.rs` - segment list drives per-frame layout selection in the compositor

## EditDoc

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditDoc {
    pub version: u32,
    pub trim: Trim,
    pub cuts: Vec<Cut>,
    pub zooms: Vec<Zoom>,
    pub speed: Vec<Speed>,
    pub layout: Vec<LayoutSeg>,
    pub settings: crate::settings::model::Settings,
}
```

Root of `edit.json`. Carries the complete editor state for one recording project.

- `version` - *schema version, currently always `1`; reserved for future migration guards.*
- `trim` - *clip in/out bounds; the only time bounds that affect the exported file's duration.*
- `cuts` - *ordered list of removed spans within the trim window.*
- `zooms` - *ordered list of zoom events; `api.rs` manages ids; the renderer tolerates any order.*
- `speed` - *ordered list of speed-change segments.*
- `layout` - *ordered, non-overlapping layout segments covering `[0, trim.out_ms]`.*
- `settings` - *snapshot of the user's `Settings` at the time the doc was seeded; preserves the zoom config and theme for a re-render even if the user later changes settings.*

### Used by

- `src-tauri/src/edit/api.rs` - `apply` mutates it; `metrics` reads it
- `src-tauri/src/edit/commands.rs` - all three Tauri commands return or accept `EditDoc`
- `src-tauri/src/edit/seed.rs` - `load_or_seed` and `build_default` construct it
- `src-tauri/src/export/fromedit.rs` - consumed to drive the compositor
- `src-tauri/src/ai/plan.rs` / `src-tauri/src/ai/commands.rs` - AI director reads and patches it

## EditDoc::save

```rust
pub fn save(&self, path: &std::path::Path) -> std::io::Result<()>
```

Serializes the doc to pretty-printed JSON and writes it atomically to `path`.

### Inputs

- `&self` - the doc to persist. *Why pretty-print:* `edit.json` is human-readable and diff-friendly in git.*
- `path: &std::path::Path` - destination file. *Why `Path` not `ProjectPaths`:* the method is on the model, which has no knowledge of project layout; callers supply the resolved path.*

### Returns

`Ok(())` on success; `Err(io::Error)` on serialization or write failure.

### Behaviors

- `round_trip_save_load` - a full `EditDoc` with all fields populated serializes and deserializes without data loss.

## EditDoc::load

```rust
pub fn load(path: &std::path::Path) -> Option<EditDoc>
```

Reads and deserializes `edit.json` at `path`. Returns `None` on any error.

### Inputs

- `path: &std::path::Path` - file to read. *Why `Option` rather than `Result`:* callers treat missing-or-corrupt as "seed needed", not an error, so a silent `None` matches the intended flow.*

## EffectKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight }
```

The kind of an editable effect region. Serializes lowercase (`"spotlight"`) to match the TS `EffectKind`. The set grows over phases (video FX, captions later).

## EffectRegion

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion { pub id: String, pub kind: EffectKind, pub start_ms: u32, pub end_ms: u32 }
```

An editable effect region on the timeline (v1: Spotlight). `EditDoc.effects` is a `Vec<EffectRegion>` with `#[serde(default)]` for back-compat (a pre-existing `edit.json` without `effects` loads). Params default from settings for now; at export, `fx_state::spotlight_region_alpha` fades a Spotlight region in/out over its span and unions it with the settings + hotkey-hold spotlight.

### Returns

`Some(EditDoc)` if the file exists and is valid JSON; `None` on any I/O or parse failure.

### Behaviors

- `load_missing_path_is_none` - a path that does not exist returns `None`.
- `partial_json_fills_defaults` - JSON with only a `zooms` key fills all other fields from `Default`.
- `zoom_target_fixed_serializes_with_xy` - `ZoomTarget::Fixed` round-trips with `"fixed"`, `"x"`, and `"y"` keys present.
