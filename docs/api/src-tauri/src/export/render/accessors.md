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

## FrameRenderer::spans

```rust
pub fn spans(&self) -> &[SourceSpan]
```

The take's source spans (`render::spans`) - one full-canvas span unless a mid-take display switch cropped it. Exposed so `preview_layouts` hands the editor the very rects the export crops by, instead of the editor deriving a second set from `sync.json`.

## FrameRenderer::span_fit

```rust
pub fn span_fit(&self, src: RectF) -> (f32, f32)
```

How much smaller (w, h) a span's screen panel is than the full-canvas one, as ratios about the panel centre: `inset_rect` at the span's own size over `inset_rect` at the canvas's. The live TS preview scales its layout-resolved screen rect by this to give a switched-to display its own aspect without re-running any pose math of its own; it is exact for the inset-based presets, and the paused stage shows the export's own frame either way (`useExactFrame`).

## FrameRenderer::time_map

```rust
pub fn time_map(&self) -> &TimeMap
```

The clip-to-output clock map built from the doc's trim, cuts and speed spans in `EditState::load` (`export/remap.md`). The exporter reads its `frame_plan`; `preview::walk_to` and `camera_track` walk the same plan.

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

## FrameRenderer::resolve_seg

```rust
pub fn resolve_seg(&self, seg: &crate::edit::model::LayoutSeg) -> crate::export::scene::Scene
```

One `LayoutSeg`'s resolved `Scene` - its own poses when it carries an `arrangement`, else its preset's `Scene` - through `scene::layout::resolve_seg_scene` (`scene/layout.md`), the exact function `LayoutTrack::from_segs` uses per segment when the export runs. Lets `preview_layouts` (T34 L2) report a posed segment's true panels to the editor preview with no second pose-math path to drift out of sync with the export.

## FrameRenderer::inset_w_frac

```rust
pub fn inset_w_frac(&self) -> f32
```

The reference inset width `cursorset::draw` normalizes the synthetic cursor's on-screen size against (as a fraction of `out_w`) - the width `inset_rect` (`coordmap.md`) computes for this renderer's OWN base `Layout` (fixed pad/scale, NOT a per-preset appearance value: see `composite_at`'s `inset_rect(self.sw, self.sh, &self.layout)` call in `mod.md`), so a screen panel narrower than this reference gets a proportionally smaller cursor - exactly like a small PiP screen shrinks it in the export.

### Returns

`f32` - `inset_rect(self.sw, self.sh, &self.layout).2 as f32 / self.layout.out_w.max(1) as f32`. Not directly unit-tested: it is a plain field read plus a call to the already-tested `inset_rect` (`coordmap.md`), so there is no new arithmetic here to pin.

### Used by

- `src-tauri/src/export/preview/preview_layouts.rs` - `preview_layouts` reports this as `LayoutPresets.inset_w`, letting the editor preview compute the SAME `panel` scale factor the export's `cursorset::draw` does (`cursorPanel.ts`'s `panelFactor`), with no second formula to drift out of sync.
