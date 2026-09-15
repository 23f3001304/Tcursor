# src-tauri/src/export/overlay.rs

The webcam overlay geometry (`OverlayShape`, `OverlayPos`, `OverlayLayout`), moved out of `export/types.rs` verbatim for headroom (2026-09-15). `export::types` re-exports all three, so every existing path still resolves.

## OverlayShape

```rust
#[derive(Clone, Copy, Debug)]
pub enum OverlayShape { Circle, Rounded { frac: f32 }, Rect }
```

Shape of the webcam/camera overlay panel.

- `Circle` - radius = `min(w, h) / 2`. Default.
- `Rounded { frac: f32 }` - corner radius = `frac * min(w, h)`. *Why a fraction:* stays proportionate as `size_px` changes.
- `Rect` - no rounding (radius = 0).

### Used by

- `src-tauri/src/export/scene/mod.rs` - `panel_radius` dispatches on this to compute the camera panel radius.

## OverlayPos

```rust
#[derive(Clone, Copy, Debug)]
pub enum OverlayPos { BottomLeft, BottomRight, TopLeft, TopRight, Custom { x: u32, y: u32 } }
```

Anchor corner for the webcam overlay relative to the output canvas.

- `BottomLeft` - default: margins from the bottom-left corner.
- `BottomRight, TopLeft, TopRight` - other corners, each applying the same `margin_x_px` / `margin_y_px`.
- `Custom { x, y }` - absolute pixel coordinates ignoring margins.

### Used by

- `src-tauri/src/export/scene/mod.rs` - `bubble_rect` maps this to a concrete `RectF`.

## OverlayLayout

```rust
#[derive(Clone, Copy, Debug)]
pub struct OverlayLayout {
    pub shape: OverlayShape, pub pos: OverlayPos,
    pub size_px: u32, pub width_px: u32,
    pub margin_x_px: u32, pub margin_y_px: u32, pub enabled: bool,
    pub ring_px: u32, pub ring_color: [u8; 3],
}
```

All overlay (webcam) layout parameters.

- `shape: OverlayShape` - overlay mask shape. Default: `Circle`.
- `pos: OverlayPos` - corner placement. Default: `BottomLeft`.
- `size_px: u32` - camera panel HEIGHT in output pixels. Default: 420.
- `width_px: u32` - camera panel WIDTH in output pixels. Default: 420 (== `size_px`; only diverges when `ModeAppearance.cam_aspect` is `Wide`, giving `round(size_px * 16/9)`). *Why a separate field, not derived at draw time:* the width:height ratio is a per-mode setting (`CamAspect`), so it is resolved once here alongside every other pixel value, the same as `size_px`.
- `margin_x_px: u32` - horizontal inset from the canvas edge. Default: 80.
- `margin_y_px: u32` - vertical inset from the canvas edge. Default: 80.
- `enabled: bool` - whether the overlay is drawn at all. Default: `true`. *Why kept here:* a disabled overlay still contributes its rect for cross-dissolve transitions; the `alpha` in the resolved `Panel` drops to 0 instead.
- `ring_px: u32` - width in output pixels of an optional colored ring/border drawn just inside the panel edge. Default: `0` (no ring). *Why px, not fraction:* resolved once here from `ModeAppearance.cam_ring`'s fraction-of-min-side, same pattern as every other geometry field.
- `ring_color: [u8; 3]` - RGB 0..255 of the ring. Default: `[0, 0, 0]` (unused when `ring_px == 0`).

### Used by

- `src-tauri/src/export/scene/mod.rs` - `resolve` passes `&OverlayLayout` to `bubble_rect` and `panel_radius`, and copies `ring_px`/`ring_color` onto the resolved camera `Panel`.
- `src-tauri/src/settings/appearance.rs` - `overlay_for` constructs this from user settings.
