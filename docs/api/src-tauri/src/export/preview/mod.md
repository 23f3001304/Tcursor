# src-tauri/src/export/preview/mod.rs

Single-frame preview engine + the warm renderer cache. Renders one composited output frame at an arbitrary scrub instant from `edit.json`, reusing `FrameRenderer` (render.rs) so the preview is byte-faithful to the export, and exposes the export background as an image. It also holds the wire encoders (`png_encode`, `jpeg_encode`, `base64_encode`) that turn a composited frame into a `data:` URL. The lightweight metadata commands the editor uses for smooth playback (camera curve, layout, clicks, proxy) live in `preview_track.rs`; both share the `with_warm` cache helper (in `session.rs`).

**The frame itself is next door.** `walk_to`, `decode_screen`, `clip_dissolve` and `composite_frame`, plus the `PreviewAt` clock the commands address a frame in and its resolver `at_instants`, moved to the sibling `compose.rs` in Batch 4 (clips) - see `compose.md`. This file keeps the commands, `build_renderer` and the encoders, imports `composite_frame`, and re-exports `PreviewAt` and `at_instants` so callers still spell them `crate::export::preview::PreviewAt`.

**The warm renderer cache.** `Cached`, `PreviewSession` (with `has_webcam`), `with_warm` and `with_warm_app` live in the sibling `session.rs` - see `session.md`. `PreviewSession` and `with_warm_app` are re-exported from here so every preview command still imports them from `crate::export::preview`; `with_warm` itself is no longer re-exported, because since Batch D every command goes through `with_warm_app`, which resolves both `PreviewSession` and `Arc<Platform>` from the `AppHandle` the command already has. They live there because the lock-discipline rewrite (build and render outside the cache mutex; `has_webcam` off the mutex entirely) wanted its own unit tests, which live in `session_tests.rs`.

## render_preview

```rust
pub fn render_preview(paths: &ProjectPaths, time_ms: u32, system: &dyn SystemPort) -> Result<Vec<u8>>
```

Renders one composited frame at `time_ms` (uncached: builds a fresh renderer via `build_renderer`) and returns PNG bytes at the resolved preview size. Not currently called by any Tauri command (`preview_frame`, below, uses the warm cache instead) - kept as the uncached entry point, and its one caller is `exporter.rs`'s `preview_frame_bench`.

**It still takes CLIP ms**, where the command beside it moved to output time in Batch 4: it wraps its argument as `PreviewAt::Clip(time_ms)` and `at_instants` runs `out_of` on it, which is exactly what `walk_to` used to do for itself. The bench measures a decode at a source instant and has no editor timeline to speak from, so nothing about its call changed.

### Inputs (what, and why it is needed)

- `paths: &ProjectPaths` - project folder. *Why:* all asset paths (video, webcam, events, edit.json) are derived from it.
- `time_ms: u32` - preview scrub position in ms from the start of the recording. *Why:* the caller knows the editor timeline position; the function converts it to an output frame index internally.
- `system: &dyn SystemPort` - forwarded to `build_renderer` for the capture fps and the desktop's dark preference. Its only caller, `exporter.rs`'s `preview_frame_bench`, builds a `platform::current()` for it.

### Returns

`Result<Vec<u8>>` - PNG bytes of the composited frame, sized to the resolved preview canvas (`PREVIEW_LONG_EDGE`, following the doc's aspect). Errors if `FrameRenderer::new` fails (missing files), if the screen decoder cannot be spawned, if `time_ms` is past the end of the video, or if in-process PNG encoding (`png_encode`) fails.

### Implementation

1. `build_renderer(paths, system)` calls `FrameRenderer::new(paths, Layout::default(), fps, Resolution::Source, Some(PREVIEW_LONG_EDGE), system)` - the doc's `aspect` is resolved against the true source dims then downscaled to the `PREVIEW_LONG_EDGE` (1280px) budget (`Layout::resolve`), so the preview frame always matches the export's aspect proportionally. `Resolution::Source` is a no-op here (the export resolution setting only applies to the export build).
2. Compute `k_target = time_ms as u64 * OUT_FPS / 1000`. Fast-forward the camera sim by calling `step_camera` for every `j` in `0..=k_target` at `video_start + j * 1000 / OUT_FPS`, each with `OUT_STEP_MS` as the frame period (the exact 16.667ms, never the rounded timestamp delta - see `render/mod.md`). This must ascend because `CameraSim` and the cursor index only move forward. The last returned `FramePose` is the preview pose. Cost: arithmetic only, no I/O.
3. Spawn a `RawDecoder` on `paths.video()` seeked to the source instant (the screen file's frame 0 is `video_start`, so it is the right offset), read one frame into a `screen_bytes`-sized buffer; bail if the read hits EOF (time past end of video).
4. If `paths.webcam().exists()`, spawn a `RawDecoder` on the webcam seeked to `video_start + time_ms` (export pre-seeks the webcam by `video_start`, so its file-time is shifted) with `cover_scale = Some((webcam_w, webcam_h))` - the same SOURCE-aspect decode box the export uses (`render::meta::webcam_box`; each panel cover-crops it at composite time), so the preview and the export never disagree about the webcam's framing - read one frame; else `webcam = None`.

   **A webcam read failure here is deliberately NOT fatal to the frame.** Both EOF and a hard decode failure just leave the buffer as it was (and log a line): `webcam.webm` routinely ends before `video.mp4`, so an `-ss` past its end is an everyday scrub near the end of a clip, and failing the whole preview frame for it would make the last seconds of such a project un-scrubbable. This is the one place the honest-failure rule (`RawDecoder::classify_end`) is deliberately relaxed - a preview frame is not a deliverable, whereas the EXPORT surfaces the same failure as an `export-warning` and a screen failure as a hard `export-error`.
5. Call `renderer.composite_at(&pose, &screen_buf, webcam_ref, &mut bgra)` to write a BGRA buffer into `bgra`.
6. Call `png_encode(bgra, meta.out_w, meta.out_h)` to produce PNG bytes in-process via the `png` crate (no ffmpeg subprocess involved), at the renderer's resolved size.

Steps 2 to 5 are `compose.rs`'s `composite_frame`; see `compose.md` for the pose walk, the dissolves and the webcam box.

## preview_frame

```rust
#[tauri::command]
pub async fn preview_frame(folder: String, out_ms: u32, app: tauri::AppHandle) -> Result<String, String>
```

Tauri IPC command: renders one preview frame (via the warm cache) and returns a JPEG data URL - the export's own frame at that instant. **Why it exists (owner ruling 2026-09-14: the export is the reference):** the stage shows this frame whenever playback pauses or a scrub settles, so what the owner looks at while editing IS a frame of the export, whatever the live canvas approximated a moment earlier.

**It takes OUTPUT ms since Batch 4** (`PreviewAt::Out(out_ms)`), where it used to take clip ms. A clip list can reorder the recording, and after a reorder one source instant is shown twice while `out_of` answers with the first showing, so a playhead resting in the second one fetched the wrong frame: the wrong overlays, the wrong camera, the wrong dissolve. The stage already computes `tOut`, so it hands over the instant it actually wants and `at_instants` runs `clip_of` for the decoder - the identity inside a segment, so an unsplit project's paused frame does not move. The full argument is in `compose.md`.

**Inside a clip dissolve the frame carries the blend.** When the pose's `clip_mix` is set, `compose.rs`'s `clip_dissolve` decodes the outgoing clip's latched frame and blends it under the incoming one through `screen_mix::blend_into` at the incoming clip's weight, the same primitive and the same alpha the export's frame loop uses, so a frame paused inside a dissolve window looks like the export's frame. The export holds that outgoing frame from its own walk; the preview renders one instant and holds nothing, so it pays one extra ffmpeg seek on the frames inside a window, at the instant `latched_ms` names rather than `clip_of(prev_out_ms)` (ruling B4-R16, argued in `compose.md`).

**Off the main thread (Task 41 sweep correction).** `async fn` + `spawn_blocking`, same freeze mechanism as `ai::commands` (Task 40) and `thumbs.rs`/`preview_track.rs` (Task 41): `render_frame` calls `RawDecoder::spawn` (screen, and webcam when present) which shells out to `ffmpeg` and blocks on its stdout pipe until the seeked frame decodes - a real blocking subprocess call, not in-process math. An earlier T41 sweep incorrectly grouped this command with `camera_track`/`preview_layout`/`click_track` (`preview_track.rs`) as "pure math/cache reads" and left it sync - those three genuinely are pure math (no decode, see their own docs); this one is not. (Sweep-2 Task 1 has since converted those three too - not because their own bodies decode, but because the `with_warm` cold path underneath every one of them does.) Because `session: tauri::State<'_, PreviewSession>` can't be moved into `spawn_blocking` (its lifetime isn't `'static`), the command instead takes `app: tauri::AppHandle` (`'static`, `Clone`, `Send`) and re-derives the same managed-state handle inside the blocking closure via `app.state::<PreviewSession>()` (`tauri::Manager`).

**Called by `useExactFrame`** (`src/editor/hooks/stage/useExactFrame.ts`, via `previewFrame` in `src/shared/ipc/preview.ts`) whenever the stage's playhead rests: `wantsExact` asks for it once playback is paused, the draft flag is clear, and the folder and the rounded `outMs`/edit-generation key have changed, and the effect fetches after a `SETTLE_MS` (160 ms) debounce so a scrub in progress does not fire one request per tick. It was uncalled from the Task 41 sweep until 2026-09-14, back when the editor's M3 preview played the recording natively via `<video>` (see `Editor.md`) with no per-frame PNG fetch anywhere. It was converted to `spawn_blocking` anyway per "dead-or-not, it must not be a landmine", and Batch 4 gave it the live caller that guard was written for.

### Inputs (what, and why it is needed)

- `folder: String` - absolute path to the project directory. *Why:* the frontend holds the folder path from `stopRecording`; it is the stable identity for a recording session across Tauri calls.
- `out_ms: u32` - scrub position in OUTPUT ms (`outMs` over the wire). *Why:* the editor timeline drives this, and it is the only clock that names one showing of a reordered instant; the frontend passes `tOut`, which its stage already has.
- `app: tauri::AppHandle` - resolves the `PreviewSession` managed state from inside the `spawn_blocking` closure (see above).

### Returns

`Result<String, String>` - on success, a `data:image/jpeg;base64,...` data URL ready for use in an `<img>` `src` attribute. On error, a human-readable error string that the frontend can display. A `spawn_blocking` join failure also maps to `Err(String)`, same shape as every other failure this command can return.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking(move || { ... })`, `.await`ed then `?`-unwrapped:

1. `app.state::<PreviewSession>()` re-derives the managed-state handle.
2. Via `with_warm`, call `composite_frame` at `PreviewAt::Out(out_ms)` and the cached size, then `jpeg_encode`.
3. Base64-encode the JPEG bytes with the local `base64_encode` helper (RFC 4648 alphabet, no line breaks).
4. Prefix with `"data:image/jpeg;base64,"` and return.

## preview_bg

```rust
#[tauri::command]
pub async fn preview_bg(folder: String, app: tauri::AppHandle) -> Result<String, String>
```

Returns the export background (the BGRA mesh/gradient the compositor draws under the screen) as a PNG data URL, so the editor's canvas preview paints the exact same background the export uses instead of an approximate gradient.

**Now off the main thread (sweep-2 Task 1, superseding the Task 41 "stays sync" verdict).** The earlier verdict was right about the warm path - `with_warm`'s fast path + `accessors::bg` (a plain field read) + an in-memory `png_encode`, no subprocess - and wrong about what that path costs. Two things it did not weigh: the COLD path underneath it still runs `FrameRenderer::new` (ffprobe/ffmpeg subprocesses + wgpu init), which Task 41 explicitly deferred as "a broader seam this fix doesn't attempt to close" and which this task closes; and the "in-memory" encode is a full-size (1280x720 by default) BGRA->RGBA swizzle plus a zlib deflate plus a hand-rolled base64 of the result - several MB of work - which `useEditorData`'s `previewBg` effect re-runs on **every pointermove of every Background-panel slider**, because it is keyed on `JSON.stringify(doc.settings.background)`. `async fn` + `spawn_blocking`, taking `app: tauri::AppHandle` instead of `tauri::State<'_, PreviewSession>` for the usual lifetime reason (see `preview_frame`); the JS call is unchanged.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) from inside the `spawn_blocking` closure. *Why:* the background is decoded once with the renderer; reusing the cache avoids rebuilding it.

### Returns

`Result<String, String>` - a `data:image/png;base64,...` URL of the background. Errors (as a string) if the renderer cannot be built or the PNG encode fails; a `spawn_blocking` join failure maps to the same shape.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking`:

1. `app.state::<PreviewSession>()` re-derives the managed-state handle.
2. Via `with_warm`, PNG-encode `renderer.bg()` at the cached `out_w` x `out_h`.
3. Base64-encode and prefix with `"data:image/png;base64,"`.

## PREVIEW_LONG_EDGE

```rust
const PREVIEW_LONG_EDGE: u32 = 1280;
```

The preview canvas long-edge budget in pixels. 1280 matches the old hardcoded 16:9 preview canvas (1280x720) exactly, so a default `Source` aspect on a 16:9 recording composites at an unchanged size; other aspects (or a non-16:9 source under `Source`) get a proportionally-sized canvas instead.

## build_renderer

```rust
fn build_renderer(paths: &ProjectPaths, system: &dyn SystemPort) -> Result<(FrameRenderer, RenderMeta)>
```

Builds a fresh preview renderer downscaled to `PREVIEW_LONG_EDGE`, following the doc's chosen `aspect` exactly via `FrameRenderer::new(paths, Layout::default(), fps, Resolution::Source, Some(PREVIEW_LONG_EDGE), system)` - the aspect is resolved against the true source dims then scaled to the long-edge budget, so the preview always matches the export's aspect proportionally. `Resolution::Source` is passed (rather than a user-chosen resolution) because the export resolution setting only applies to the export build, not the preview.

`fps` is `system.primary_refresh_hz().min(60)` - the same number `exporter::export` computes for the export build, from the same port, which is what keeps the preview's timeline denominator identical to the export's.

## jpeg_encode

```rust
pub(crate) fn jpeg_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>>
```

JPEG-encodes a BGRA buffer in process with the `jpeg-encoder` crate (0.7, no transitive dependencies) at `JPEG_QUALITY` = 90, taking BGRA directly so there is no swizzle pass. Until 2026-09-15 this staged the frame to a temp file and spawned ffmpeg (`-f rawvideo -pix_fmt bgra ... -q:v 3 -f mjpeg -`), which the readability doc (section 4.1) measured at about 220 ms of a roughly 350 ms settled-frame call; the in-process encoder removes the temp-file write and the process spawn, and `preview_bg` (which runs at pointer-move rate) gains the same. *Why not `png_encode`:* the stage draws this frame the moment playback pauses or a scrub settles, and a 1280-wide PNG deflate in a debug build costs more than the render itself; the JPEG is a tenth of the bytes over IPC. `u16::try_from` bounds the dimensions at 65535 (the format's own limit); errors on an encoder failure or fewer than four bytes. Test: `tests::a_bgra_buffer_encodes_to_a_jpeg` in the sibling `mod_tests.rs` (no ffmpeg needed any more).

## JPEG_QUALITY

`90` - the encoder's quality, chosen to match what ffmpeg's `-q:v 3` produced for the same frames.

## png_encode

```rust
pub(crate) fn png_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>>
```

PNG-encodes a BGRA buffer in memory (swizzles to RGBA, then the `png` crate at 8-bit RGBA). Used where the bytes must be exact rather than small: `render_preview` (the `preview_frame_bench` output) and `preview_bg`.

## base64_encode

```rust
pub(crate) fn base64_encode(input: &[u8]) -> String
```

Base64 (RFC 4648 alphabet, `=` padding, no line breaks), for the `data:` URLs the preview commands return. Local rather than a dependency: it is ~15 lines and runs once per reply.
