# src-tauri/src/export/preview/compose.rs

The one-frame compositor behind every preview command: resolve the instant the caller asked for, walk the camera to it, decode the screen frame (and, inside a clip dissolve or a mid-take display switch, the outgoing one), then hand the lot to `FrameRenderer::composite_at`. Split out of `mod.rs` in Batch 4 (clips), when the dissolve took that file to 291 lines against a 280 budget; `mod.rs` keeps the Tauri commands, the warm-cache entry points and the wire encoders, and imports `composite_frame` from here, re-exporting `PreviewAt` and `at_instants` so callers still spell them `crate::export::preview::PreviewAt`. The sibling `preview_track.rs`, `segments_audio.rs` and `segments_webcam.rs` were split off the same file for the same reason.

**Why the preview is addressed by OUTPUT time.** A clip list can reorder the recording, and after a reorder one source instant can be shown twice: `TimeMap::out_of` answers with the FIRST showing, so a playhead resting in the second one used to fetch the first - the wrong overlays, the wrong camera, the wrong dissolve. The editor's stage already computes `tOut` for every tick (`useStageEngine.ts`), so the command takes the instant it actually wants and resolves the source instant with `clip_of`. Inside a segment `clip_of(out_of(t)) == t`, so an unsplit project's paused frame does not move (pinned by `an_output_instant_round_trips_inside_a_segment`).

## PreviewAt

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewAt { Clip(u32), Out(u32) }
```

Which clock the caller is speaking in. `Out(o)` is the editor's output time, what the `preview_frame` command takes since Batch 4. `Clip(t)` is source time, kept for `render_preview` (the uncached entry point behind `preview_frame_bench`), which measures a decode at a source instant and has no timeline to speak from.

## at_instants

```rust
pub fn at_instants(map: &TimeMap, at: PreviewAt) -> (u32, u32)
```

Resolves a `PreviewAt` into the `(out_ms, clip_ms)` pair the rest of the frame needs: `out_ms` drives `walk_to` (the camera, the overlays, the dissolve), `clip_ms` is where the screen decoder seeks. `Clip(t)` answers `(map.out_of(t), t)`, exactly what `walk_to` used to compute for itself; `Out(o)` answers `(o, map.clip_of(o))`. Tests: `compose_tests.rs`, all three shapes, including the reordered `clips_fixture` where `Out(0)` resolves to source 6000 and `Clip(750)` still answers with the first showing at 5250.

## latched_ms

```rust
fn latched_ms(map: &TimeMap, prev_out_ms: u32, fps: u64) -> Option<u32>
```

The source instant of the frame the EXPORT latched at a clip join, from the `ClipMix::prev_out_ms` the pose carries. `None` when the plan is empty (everything cut), which cannot happen while a pose carries a `clip_mix`.

**Why not `clip_of(prev_out_ms)` (controller ruling B4-R16).** `decode_screen` seeks with `-ss` before `-i`, so ffmpeg returns the first frame whose pts is at or AFTER the seek. `prev_out_ms` is one millisecond before the window opens and almost never sits on a frame, so `clip_of` of it lands on the frame after the one the export holds, and when the outgoing clip runs to the end of the recording there may be no frame at or after it at all - the decode fails and the paused frame silently shows no dissolve. The export's latch is `plan[j_prev]` with `j_prev = prev_out_ms * fps / 1000` (the same integer division `walk_to` uses for `j_target`; for a window opening at plan index P it is exactly P - 1), and the instant that lands ON that frame is `plan[j_prev] * 1000 / fps`, which Task 2 proved frame exact. Worked on the shared `clips_fixture` at 10 fps: the second clip opens at plan index 49, so `prev_out_ms` is 4899, `j_prev` is 48, `plan[48]` is 89 and the answer is 8900 ms. Worked on an in-order split (full 2507 ms, clips 0..1210 and 1210..2507) at 60 fps, where the join is not frame aligned in milliseconds: the first clip ends on an inner join so its frames are 0..72 (`ceil(1210 * 60 / 1000) - 1`), the window opens at plan index 73 = 1216 ms, `prev_out_ms` is 1215, `j_prev` is 72, `plan[72]` is 72 and the answer is 1200 ms - where `clip_of(1215)` would have said 1215 and ffmpeg would have handed back frame 73. Both are pinned in `the_outgoing_frame_is_the_one_the_export_latched_not_the_one_after_it`, `fps` being a parameter so the pins can be worked by hand at 10 fps instead of `OUT_FPS`.

## walk_to

```rust
pub(crate) fn walk_to(r: &mut FrameRenderer, video_start: u64, out_ms: u32) -> FramePose
```

Step the camera along the renderer's frame plan up to the output frame `out_ms` falls in (`out_ms * OUT_FPS / 1000`, clamped to the last plan entry), through `FrameRenderer::walk_plan` with a no-op body, so the one-shot preview frame's camera, spans, display-switch mix and clip mix are exactly the export's at that frame. It takes OUTPUT ms since Batch 4 and no longer calls `out_of` itself; `at_instants` did that conversion for the whole frame, so there is one answer to which instant is being drawn instead of two. With everything cut there is no plan and the camera is stepped once at output time 0 instead.

## decode_screen

```rust
fn decode_screen(paths: &ProjectPaths, meta: &RenderMeta, time_ms: u32, buf: &mut [u8]) -> Result<()>
```

Seek-decodes one screen frame at `time_ms` (source time) into `buf` as nv12 at the renderer's own crop. Factored out because `composite_frame` decodes up to THREE frames: the one at `time_ms`; the frame at `SpanMix::hold_ms` when the instant is inside a mid-take display switch; and, inside a clip dissolve, the frame `latched_ms` names. Each of those extra ones is the picture the EXPORT holds at that instant, so a ghost image in a paused preview frame is the export's own. Only the first is fatal: the other two drop their dissolve rather than fail the scrub, since only the export treats a screen decode as a deliverable.

## clip_dissolve

```rust
fn clip_dissolve(map: &TimeMap, meta: &RenderMeta, paths: &ProjectPaths, pose: &FramePose, cur: &[u8]) -> Option<Vec<u8>>
```

The paused frame's half of the clip cross-dissolve, the mirror of the export's blend arm in `pipeline/frame_loop.rs`. `None` when the pose carries no `clip_mix` (every frame outside a dissolve window), when the plan is empty, or when the outgoing frame will not decode; otherwise the incoming frame `cur` with the outgoing clip's latched frame blended under it.

Where the export already HOLDS the outgoing frame (Task 2 latches it at every join) the preview renders one instant and holds nothing, so it decodes that frame on the spot: one extra ffmpeg seek, and only on the frames inside a dissolve window. Everything else is the export's arm verbatim - `screen_mix::blend_into` over `pose.scene.src` for both source and destination rect, at `ClipMix::alpha`, which is the INCOMING clip's weight and runs on `camera::ease` (ruling B4-R1) because Task 3 resolved it that way; nothing here evaluates a curve. A failed decode is logged (`[PREVIEW] outgoing clip frame at ...`) the way the webcam failure beside it is and falls back to the unblended frame, rather than being swallowed by an `.ok()`.

## composite_frame

```rust
pub(super) fn composite_frame(renderer: &mut FrameRenderer, meta: &RenderMeta, paths: &ProjectPaths, at: PreviewAt) -> Result<Vec<u8>>
```

One composited BGRA frame at `at`, for `render_preview` (which passes `Clip`) and the `preview_frame` command (which passes `Out`).

1. `reset_camera`, then `at_instants` for the `(out_ms, clip_ms)` pair and `walk_to(out_ms)` for the pose.
2. `decode_screen` at `clip_ms` into the screen buffer; if `clip_dissolve` returns a blend, that becomes the screen buffer instead.
3. `pose.mix` (a mid-take display switch) decodes its own held frame at `clip_of(hold_ms)`, passed to `composite_at` as `prev` so the compositor dissolves the panel geometry. This one still goes through `clip_of` because `SpanMix::hold_ms` is already the held frame's own output instant and not one millisecond before a join.
4. The webcam, when `webcam.webm` exists, at `video_start + clip_ms` through the export's own `webcam_box` decode box. A read failure here logs and leaves the buffer as it was: `webcam.webm` routinely ends before `video.mp4`, and failing a whole preview frame for it would make the last seconds of a project un-scrubbable (see `mod.md` on the honest-failure rule this relaxes).
5. `renderer.composite_at(&pose, &screen_buf, prev, wc_ref, &mut bgra)`.

**What is pinned and what is not.** The instants are unit tested (`at_instants`, `latched_ms`); the body needs a real recording, ffmpeg and a GPU, so the dissolve branch is held by `the_paused_dissolve_decodes_the_latched_frame_and_blends_it_at_the_incoming_weight`, a source pin over this file's own text in the shape of `fx_seam_tests.rs`: `clip_dissolve` must find the latched instant, decode it and then blend it, must not reach for `clip_of` (the B4-R16 regression), must blend at `m.alpha`, and `composite_frame` must call it. Whether the paused frame and the export's frame are the same PICTURE is the owner's look pass.
