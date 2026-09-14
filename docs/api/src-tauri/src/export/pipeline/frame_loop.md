# src-tauri/src/export/pipeline/frame_loop.rs

The exporter's per-frame loop, moved out of `exporter.rs` (at the size cap) when the time remap replaced "every recording frame between the trim bounds" with "every output frame of the plan". A private child module of `exporter` (`#[path]`), so it shares `exporter_report`.

## Pipes

```rust
pub(super) struct Pipes { pub spipe: ScreenPipe, pub wpipe: Option<WebcamPipe>, pub bgpipe: Option<BgPipe>,
    pub last_webcam: Option<(Vec<u8>, u32, u32)>, pub wc_fail: Option<String>, pub wc_frames: u64,
    pub held: Option<(usize, Vec<u8>)> }
```

`held` is the last screen frame of the span before a mid-take display switch, with that span's index: what the switch's cross-dissolve blends FROM (`render::spans`). The loop latches it on the one output frame `FramePose::hold` names - copied while the decode buffer is still borrowed, stored after the composite releases it - and only uses it while `FramePose::mix` names that same span, so a cut that skipped the latch frame drops the dissolve instead of blending a stale picture. `None` all take long for a recording that never switched display, which costs that (overwhelmingly common) case nothing.

The three decoders plus the held webcam frame and the webcam failure bookkeeping that `webcam_warning` reads after the loop. `advance` decodes ONE recording frame on the screen pipe (which then holds it) and the webcam pipe (holding its latest frame; a shorter webcam stream freezes on its last). `feed_bg` pulls one background-video frame per OUTPUT frame: the background runs on the output clock, so its frame index is the output frame index.

## Timing

```rust
pub(super) struct Timing { pub dec: u128, pub comp: u128, pub send: u128 }
```

Microseconds spent decoding, compositing and waiting on the encoder channel, for `exporter_report::log_timing`.

## Clock

```rust
pub(super) struct Clock { pub video_start: u64, pub out_fps: u64 }
```

## run

```rust
pub(super) fn run(r: &mut FrameRenderer, pipes: &mut Pipes, plan: &[u64], video: &Path, clock: &Clock,
                  out_dims: (u32, u32), out_pool: &BufPool, tx: &SyncSender<Frame>, on_progress: &impl Fn(u8)) -> Result<(u64, Timing)>
```

Rides `FrameRenderer::walk_plan` (which steps the camera once per output frame, warms up before the first kept frame and snaps the cursor across a cut) and, inside its callback, advances the pipes by `PlanCursor::next().decodes_needed`, feeds the background, composites the held screen frame with the held webcam frame, sends the output frame with an OUTPUT-time timestamp (`video_start + j * 1000 / out_fps`), and reports progress as `j + 1` of the plan length. A decode error or a closed encoder channel stops the walk; the error, if any, is returned after it. Returns the frames actually sent and the timing split.
