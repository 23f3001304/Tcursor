# render_edit

The `edit.json`-derived slice of the render state (`EditState`), split out so the warm preview can reflect an edit by rebuilding only this cheap CPU state instead of a full `FrameRenderer::new` (which recreates the GPU device, decodes the background, probes the video, and preps the cursor sprites). Both `FrameRenderer::new` and `FrameRenderer::reload_edit` build it from the same code, so an edit reflects identically whether the renderer was freshly built or refreshed in place.

Cursor prep (`CursorPrep`) is deliberately excluded: it is edit-independent for the warm preview (only `composite_at` uses it, which the editor never calls during editing) yet dominates a full build, so keeping it out of `EditState` is what makes `reload_edit` cost microseconds.

## EditState

```rust
pub(crate) struct EditState {
    pub map: TimeMap,
    pub settings: Settings,
    pub cfg: ZoomConfig,
    pub track: SpanTrack,
    pub regions: Vec<ZoomRegion>,
    pub effects: Vec<EffectRegion>,
    pub captions: Vec<Caption>,
    pub cam_moves: CameraMoveTrack,
}
```

Everything the renderer derives from `edit.json` that a zoom/spotlight/camera-move edit can change: the settings, the zoom config, the layout track, the zoom regions (raw canvas-space anchors - `step_camera` re-anchors them per frame), the effect regions, the spoken-caption track, and the camera-moves track (Task 4). Pure CPU - no GPU, no video probe, no background decode, no cursor prep - so it rebuilds in ~microseconds.

- `captions: Vec<Caption>` - `doc.captions`, already moved onto the output clock by `remap_doc` along with each caption's own word timings. It sits on `EditState` rather than being read straight off the doc in `composite_at` so that a caption edit refreshes through exactly the path a zoom edit does - `reload_edit`, not a rebuild.
- `cam_moves: CameraMoveTrack` - `CameraMoveTrack::from_doc(&doc.camera_moves)`; empty when the doc has no `camera_moves` (the default), which is the signal `FrameRenderer::step_camera` uses to leave the scene's camera panel untouched.

## EditState::load

```rust
pub(crate) fn load(paths: &ProjectPaths, actions: &[ActionEvent], layout: &Layout, sw: u32, sh: u32,
                   shift: i64, video_start: u64, full_dur_ms: u32) -> Self
```

`track` is a `SpanTrack`: one `LayoutTrack` per SOURCE SPAN, each built at that span's own source size, because `scene::resolve` shapes the screen panel from the size it is handed. A take that never switched display has one span and therefore one track built from exactly the old `(sw, sh)` arguments. `video_start` is the recording-clock instant of the first video frame, needed only to move `sync.json`'s display-switch instants onto the clip clock before the time map turns them into output time.

**Time remap (first thing it does).** `full_dur_ms` is the clip length; the `TimeMap` is built from the raw doc's trim, cuts and speed spans, and the doc is run through `edit::remap_doc` BEFORE any track is built, so `fromedit`, `LayoutTrack`, `CameraMoveTrack` and the effects and captions lists only ever see output-clock spans and never learn about cuts or speed. The recorded layout switches used by the fallback track (`actions_on_output_clock`, which despite its name puts them on the clip clock) are mapped through `out_of` the same way. The map is kept on the state (`map`) and reaches the renderer's `time_map()`.

Loads the edit-derived state from `paths` (reads `edit.json` via `load_or_seed`). `actions` / `layout` / `sw` / `sh` are the edit-independent inputs the caller already holds (recorded action log, preview layout, probed video dims), so this touches only `edit.json`.

`shift` is `events_ms - video_start`, and exists for ONE branch: when `doc.layout` is empty the layout track falls back to the RECORDED `SetLayout` actions, which are timestamped on the event clock, while `LayoutTrack::scene_at` is sampled at `out_t` like every other doc region. The fallback therefore rebuilds the action log through `seed::actions_on_output_clock(actions, shift)` first - the same helper `seed::build_default` uses - so a fallback track and a seeded track can never land on different clocks. The normal branch (`LayoutTrack::from_segs(&doc.layout, ..)`) needs no conversion: `doc.layout` is already output time.

### Used by

`FrameRenderer::new` (once, at build time) and `FrameRenderer::reload_edit` (on every warm-preview edit).
