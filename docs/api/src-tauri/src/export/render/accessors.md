# src-tauri/src/export/render/accessors.rs

Small read-only `FrameRenderer` accessors used only by preview commands OUTSIDE the renderer (`preview.rs`, `preview_track.rs`, `preview_fx.rs`, `cursorpreview.rs`) - never by the export loop, which drives everything through `step_camera`/`composite_at` in `mod.rs` directly. Split into its own file purely to keep `mod.rs` under the size budget; these remain inherent methods on `FrameRenderer` (Rust privacy is scoped to the defining module and its descendants, and this file is a child module of `render`, so `FrameRenderer`'s private fields are visible here exactly as in `mod.rs`).

## FrameRenderer::bg

```rust
pub fn bg(&self) -> &[u8]
```

The export background buffer (BGRA, `out_w * out_h * 4` bytes) - the same mesh/gradient the compositor draws under the screen. Exposed so the editor preview (`preview_bg`) can paint the exact same background the export uses.

## FrameRenderer::has_webcam

```rust
pub fn has_webcam(&self) -> bool
```

Whether `webcam.webm` exists on disk for this recording (the private `has_webcam` field, computed once in `FrameRenderer::new`). Exposed so preview commands outside the renderer - specifically `preview_fx_overlay` via `PreviewSession::has_webcam` (`preview/mod.md`) - can gate the spotlight's camera-exclusion hole the same way `composite_at`/`fx_state::render` already do for the export.

## FrameRenderer::click_track

```rust
pub fn click_track(&self, video_start: u64) -> Vec<(u32, f32, f32)>
```

Click (mouse-down) events mapped to `(output_ms, x, y)` where `x`/`y` are 0..1 fractions of the screen content - so the editor preview can draw click ripples that match the export's click FX.

### Inputs

- `video_start: u64` - the first video frame's capture timestamp (from `RenderMeta`). *Why:* the output time inverts `step_camera`'s `ev_t = t - events_ms` offset, so a click at event time `et` shows at output `et + events_ms - video_start`.

### Returns

`Vec<(u32, f32, f32)>` - one tuple per mouse-down event whose output time is >= 0, in ascending time order. Positions come from `Cursor::clicks` (same basis as the smoothed cursor), so ripples land exactly where the cursor clicked.

## FrameRenderer::events_ms

```rust
pub fn events_ms(&self) -> u64
```

The event-time base (the private `events_ms` field), so preview commands outside the renderer can map an event timestamp to output time the same way `click_track` does.

## FrameRenderer::actions

```rust
pub fn actions(&self) -> &[ActionEvent]
```

The recorded action log (hotkey hold starts/ends), so preview commands can surface recorded effect holds (e.g. spotlight) the way `click_track` surfaces clicks.

## FrameRenderer::resolve_layout

```rust
pub fn resolve_layout(&self, id: crate::actions::model::LayoutId) -> crate::export::scene::Scene
```

This preset's `Scene`, resolved from the renderer's own appearance + dims (the same resolve `LayoutTrack` performs) - lets `preview_layouts` cross-fade between presets.
