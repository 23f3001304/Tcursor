# render_edit

The `edit.json`-derived slice of the render state (`EditState`), split out so the warm preview can reflect an edit by rebuilding only this cheap CPU state instead of a full `FrameRenderer::new` (which recreates the GPU device, decodes the background, probes the video, and preps the cursor sprites). Both `FrameRenderer::new` and `FrameRenderer::reload_edit` build it from the same code, so an edit reflects identically whether the renderer was freshly built or refreshed in place.

Cursor prep (`CursorPrep`) is deliberately excluded: it is edit-independent for the warm preview (only `composite_at` uses it, which the editor never calls during editing) yet dominates a full build, so keeping it out of `EditState` is what makes `reload_edit` cost microseconds.

## EditState

```rust
pub(crate) struct EditState {
    pub settings: Settings,
    pub cfg: ZoomConfig,
    pub track: LayoutTrack,
    pub regions: Vec<ZoomRegion>,
    pub effects: Vec<EffectRegion>,
}
```

Everything the renderer derives from `edit.json` that a zoom/spotlight edit can change: the settings, the zoom config, the layout track, the anchored zoom regions, and the effect regions. Pure CPU - no GPU, no video probe, no background decode, no cursor prep - so it rebuilds in ~microseconds.

## EditState::load

```rust
pub(crate) fn load(paths: &ProjectPaths, actions: &[ActionEvent], layout: &Layout, sw: u32, sh: u32) -> Self
```

Loads the edit-derived state from `paths` (reads `edit.json` via `load_or_seed`). `actions` / `layout` / `sw` / `sh` are the edit-independent inputs the caller already holds (recorded action log, preview layout, probed video dims), so this touches only `edit.json`.

### Used by

`FrameRenderer::new` (once, at build time) and `FrameRenderer::reload_edit` (on every warm-preview edit).
