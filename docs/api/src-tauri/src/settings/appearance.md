# src-tauri/src/settings/appearance.rs

Resolution-independent appearance settings for the five layout modes: webcam bubble shape/corner/aspect/ring and per-mode sizing expressed as fractions of the canvas. Defaults reproduce the legacy hardcoded render exactly. Pure data plus two resolver functions that map fractions to pixels at export time.

## CamShape

```rust
pub enum CamShape { Circle, Rounded, Rect }
```

The webcam bubble's shape. *Why:* lets the user pick how the camera panel is masked without separate booleans.

- `Circle` - a full pill/circle mask.
- `Rounded` - rounded corners using `ModeAppearance.cam_radius` (a fraction of the panel's min side). *Why a separate radius field:* only `Rounded` reads it.
- `Rect` - square corners.

### Used by

- `src-tauri/src/settings/appearance.rs` - `overlay_for` maps it to `export::types::OverlayShape`.
- `src-tauri/src/settings/model.rs` - reachable through the persisted `Settings`.

## CamCorner

```rust
pub enum CamCorner { BottomLeft, BottomRight, TopLeft, TopRight }
```

Which canvas corner the webcam bubble anchors to (bubble modes only). *Why:* the big-camera modes center the panel, so corner only matters for the small bubble.

### Used by

- `src-tauri/src/settings/appearance.rs` - `overlay_for` maps it to `export::types::OverlayPos`.

## CamAspect

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CamAspect { Square, Wide }
```

Webcam PANEL (PiP bubble) aspect ratio. *Why:* lets the webcam be wider than tall without touching `cam_size`, which stays the HEIGHT basis either way.

- `Square` - width == height (`cam_size`). Default; byte-identical to pre-Task-9 behavior.
- `Wide` - 16:9; width is derived from height (`round(cam_size_px * 16/9)`), height unchanged.

Only applies to the PiP bubble (`Screen`/`ScreenOnly` modes' `bubble_rect`) - the big-camera/centered-square modes (`Camera`, `CameraOnly`, `Presenter`) ignore it and always stay square (pinned by `wide_aspect_does_not_affect_big_camera_modes` in `export/scene/mod_tests.rs`).

### Used by

- `src-tauri/src/settings/appearance.rs` - `overlay_for` computes `OverlayLayout.width_px` from it.

## CamRing

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct CamRing { pub width: f32, pub color: [u8; 3] }
```

Optional colored ring/border drawn just inside the webcam panel edge.

- `width: f32` - fraction of the panel's min side (so it scales with `cam_size`, like `cam_radius`).
- `color: [u8; 3]` - RGB 0..255.

### Used by

- `src-tauri/src/settings/appearance.rs` - `ModeAppearance.cam_ring: Option<CamRing>`; `overlay_for` computes `OverlayLayout.ring_px`/`ring_color` from it (`None` -> `ring_px: 0`, no ring drawn).

## ModeAppearance

```rust
pub struct ModeAppearance {
    pub pad: f32, pub screen_size: f32, pub screen_radius: f32,
    pub cam_size: f32, pub cam_shape: CamShape, pub cam_radius: f32,
    pub cam_corner: CamCorner, pub cam_margin_x: f32, pub cam_margin_y: f32,
    pub cam_aspect: CamAspect, pub cam_ring: Option<CamRing>,
}
```

Appearance for a single layout mode. All sizes are fractions of the canvas (resolution-independent); the render maps them to pixels at resolve time. *Why fractions:* the same settings produce identical framing at 1080p or 4K.

- `pad: f32` - inset padding, fraction of canvas WIDTH (both axes). Default `0.03125` (120px at 3840 wide).
- `screen_size: f32` - scale of the screen panel about its center, 0.6..1.0. Default `1.0`.
- `screen_radius: f32` - screen-panel corner radius, fraction of canvas HEIGHT. Default `0.016`.
- `cam_size: f32` - camera panel HEIGHT, fraction of canvas HEIGHT. Default `0.1944` (the bubble). *Why height, not size:* `cam_aspect` derives the width from this when `Wide`.
- `cam_shape: CamShape` - bubble shape. Default `Circle`.
- `cam_radius: f32` - rounded-corner fraction of the panel min-side; read only when `cam_shape == Rounded`. Default `0.04`.
- `cam_corner: CamCorner` - bubble anchor corner. Default `BottomLeft`.
- `cam_margin_x: f32`, `cam_margin_y: f32` - bubble insets, fractions of canvas WIDTH/HEIGHT. Defaults `0.0208` / `0.037`.
- `cam_aspect: CamAspect` - panel width:height ratio (PiP bubble only). Default `Square` (byte-identical to pre-Task-9).
- `cam_ring: Option<CamRing>` - optional colored border just inside the panel edge. Default `None` (no ring, byte-identical to pre-Task-9). `#[serde(default)]` on the field itself (in addition to the struct-level default) so old JSON without this key deserializes to `None`.

`#[serde(default)]` so partial JSON fills missing fields from `Default` (pinned by `round_trip_and_partial_json_fill_defaults`).

### Used by

- `src-tauri/src/settings/appearance.rs` - input to `layout_for` and `overlay_for`.
- `src-tauri/src/export/scene/mod.rs`, `src-tauri/src/export/pipeline/exporter.rs` - resolve the active mode's `ModeAppearance` into render geometry.

## AppearanceSettings

```rust
pub struct AppearanceSettings {
    pub screen: ModeAppearance, pub screen_only: ModeAppearance,
    pub camera: ModeAppearance, pub camera_only: ModeAppearance,
    pub presenter: ModeAppearance,
}
```

One `ModeAppearance` per layout mode. *Why per-mode:* the big-camera modes (`camera`, `camera_only`, `presenter`) default to a large rounded square (`cam_size 0.889`, `Rounded`), while the bubble modes (`screen`, `screen_only`) default to the small circle - so a single shared block could not express both.

### Used by

- `src-tauri/src/settings/model.rs`, `src-tauri/src/settings/mod.rs` - embedded in the persisted `Settings`.
- `src-tauri/src/export/scene/layout.rs`, `export/scene.rs`, `export/exporter.rs` - read per-mode appearance during scene resolution.

## AppearanceSettings::for_id

```rust
pub fn for_id(&self, id: LayoutId) -> &ModeAppearance
```

Returns the `ModeAppearance` block for a layout mode.

### Inputs

- `&self` - the settings holding all five blocks.
- `id: LayoutId` - which mode to select. *Why take the enum:* the caller has a `LayoutId` from the layout timeline and should not know the field names.

### Returns

A borrow of the matching block (`&self.screen` ... `&self.presenter`). Total - the match is exhaustive over `LayoutId` (pinned by `for_id_maps_each_mode`).

## layout_for

```rust
pub fn layout_for(ma: &ModeAppearance, ow: u32, oh: u32) -> Layout
```

Builds the per-mode screen `Layout` (canvas dims + padding + screen scale/radius in pixels) from fractional settings.

### Inputs

- `ma: &ModeAppearance` - the resolved mode block. *Why:* supplies `pad`, `screen_size`, `screen_radius` as fractions.
- `ow: u32`, `oh: u32` - output width/height in pixels. *Why both:* `pad` scales by WIDTH, `screen_radius` by HEIGHT (so the radius reads consistently across aspect ratios).

### Returns

A `Layout` with `pad_px = round(pad * ow)`, `screen_scale = screen_size`, `screen_radius_px = screen_radius * oh`. At the default screen mode and 3840x2160 this is `pad_px 120`, `radius 34.56` (pinned by `layout_for_default_screen_is_today_px`).

## overlay_for

```rust
pub fn overlay_for(ma: &ModeAppearance, ow: u32, oh: u32, enabled: bool) -> OverlayLayout
```

Builds the per-mode webcam `OverlayLayout` (shape/position/size/width/margins/ring in pixels) from fractional settings.

### Inputs

- `ma: &ModeAppearance` - supplies `cam_shape`, `cam_corner`, `cam_size`, `cam_radius`, `cam_aspect`, `cam_ring`, margins.
- `ow: u32`, `oh: u32` - output pixels; `cam_size` and `margin_y` scale by HEIGHT, `margin_x` by WIDTH.
- `enabled: bool` - whether the overlay is drawn. *Why a parameter, not a field:* enablement is a render-time decision (e.g. no webcam recorded), separate from the styling settings.

### Implementation

1. Map `cam_shape` to `OverlayShape` (`Rounded` carries `frac: cam_radius`).
2. Map `cam_corner` to `OverlayPos`.
3. Compute `size_px = round(cam_size * oh)` (panel HEIGHT).
4. Compute `width_px`: `Square` -> `size_px`; `Wide` -> `round(size_px * 16/9)`.
5. Compute `ring_px`/`ring_color` from `cam_ring`: `Some(r)` -> `ring_px = round(r.width * min(width_px, size_px))`, `ring_color = r.color`; `None` -> `(0, [0,0,0])`.
6. Compute `margin_x_px = round(cam_margin_x * ow)`, `margin_y_px = round(cam_margin_y * oh)`.

### Behaviors

Default screen mode at 4K yields a 420px circle at (80,80) margins with `width_px == size_px == 420` and `ring_px == 0` (pinned by `overlay_for_default_screen_is_today_bubble`); default camera mode yields a 1920px rounded square, also `width_px == 1920` (pinned by `overlay_for_default_camera_is_rounded_1920`). `Wide` aspect scales only `width_px` (pinned by `overlay_for_wide_aspect_scales_width_only`); a `cam_ring` scales `ring_px` off `min(width_px, size_px)` (pinned by `overlay_for_ring_scales_from_min_side`); no `cam_ring` always yields `ring_px: 0` (pinned by `overlay_for_no_ring_is_zero`).
