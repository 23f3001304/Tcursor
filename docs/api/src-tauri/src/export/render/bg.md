# src-tauri/src/export/render/bg.rs

The renderer's background: how the static `bg` buffer is built, how an edit refreshes the `edit.json`-derived state around it, and how a VIDEO background's decoded frames get swapped in.

Split out of `render/mod.rs`, which was at its 200-line budget, rather than grown there. A child module can implement methods on its parent's type and reach its private fields, so `FrameRenderer` is unchanged - only the file boundary moved.

## BG_MESH

```rust
pub(super) const BG_MESH: &[u8] = include_bytes!("../../../assets/backgrounds/bg.jpg")
```

The legacy bundled background ("Classic"): what `BackgroundSettings.mesh == ""` renders, which is what every project saved before the wallpaper library deserializes to. Embedded, so a wallpaper needs no install-time file layout.

## build_bg

```rust
pub(super) fn build_bg(paths: &ProjectPaths, settings: &BackgroundSettings, w: u32, h: u32) -> Vec<u8>
```

`export::scene::background::build` with this renderer's two constants filled in: the embedded legacy mesh, and `paths.folder` as the project directory an `Image`/`Video` asset path is relative to. The one place those two arguments are supplied, so `FrameRenderer::new` and `reload_edit` cannot disagree about them.

## reload_edit

```rust
pub fn reload_edit(&mut self, paths: &ProjectPaths)
```

Rebuilds the `EditState` with the renderer's own `full_dur_ms`, so a cut or speed edit in the editor changes the time map (and every remapped region) on the next preview frame without a renderer rebuild.

Refresh the `edit.json`-derived state in place (zoom/layout/regions/effects/captions/cam_moves/grade/texts) WITHOUT recreating the GPU compositor or re-probing dims - this is what makes a warm preview cheap. Moved here verbatim from `render/mod.rs`, plus one addition.

The fields are copied across ONE BY ONE rather than by replacing the whole struct, because `FrameRenderer` also holds the edit-independent state (the GPU compositor, the decoded background, the cursor prep) that `EditState` deliberately excludes. So every field the renderer carries out of the edit doc needs a line here as well as on `EditState` and on `FrameRenderer`. **Anything added to `EditState` must be added here too**, or it will load correctly on a cold build and then never refresh on an edit: `self.grade = es.grade;` is why changing the look in the Background panel repaints the preview instead of taking effect only on the next export, and `self.texts = es.texts;` is the same for an edited text item.

The cursor's three settings-driven filters are live-applied here rather than waiting for a rebuild: `Cursor::set_smoothness` (Smoothness), `set_idealize` (Path Idealization) and `set_tilt` (Motion Tilt), each through its `_at` variant so switching the style to `System` on a recording with no baked OS cursor also flips the path to raw and the lean to none in the same call.

`bg` is rebuilt ONLY when `BackgroundSettings` actually changed (`PartialEq`), because the rebuild shells out to ffmpeg and every other kind of edit must stay cheap. When it does rebuild, `set_bg_dynamic` is re-asserted on the compositor: an edit can turn a still background into a video one or back, and the GPU upload rule differs between the two.

## background

```rust
pub fn background(&self) -> &BackgroundSettings
```

This renderer's background settings. Read by the exporter to decide whether to open a decode stream at all (`background::video_source`) and at what `dim`.

## swap_bg

```rust
pub fn swap_bg(&mut self, buf: Vec<u8>) -> Vec<u8>
```

Swap in one decoded VIDEO background frame, handing back the buffer it replaces so the caller can return it to its pool.

*Why a swap rather than a copy:* at 1080p this runs 60 times a second and each buffer is ~8 MB. `std::mem::replace` moves both ways and allocates nothing; the returned buffer goes straight back to `BgPipe`'s `BufPool`.

### Used by

- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`) - calls `build_bg` for the initial buffer.
- `src-tauri/src/export/pipeline/bg_pipe.rs` (`BgPipe::feed`) - `swap_bg` once per output frame.
- `src-tauri/src/export/pipeline/exporter.rs` - `background()` to decide whether a stream is needed.
- `src-tauri/src/export/preview/session.rs` - `reload_edit` on a warm preview.
