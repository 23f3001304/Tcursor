# src-tauri/src/export/pipeline/frame_loop.rs

The exporter's per-frame loop, moved out of `exporter.rs` (at the size cap) when the time remap replaced "every recording frame between the trim bounds" with "every output frame of the plan". A private child module of `exporter` (`#[path]`), so it shares `exporter_report`.

## Pipes

```rust
pub(super) struct Pipes { pub spipe: ScreenPipe, pub wpipe: Option<WebcamPipe>, pub bgpipe: Option<BgPipe>,
    pub last_webcam: Option<(Vec<u8>, u32, u32)>, pub wc_fail: Option<String>, pub wc_frames: u64,
    pub held: Option<(usize, Vec<u8>)>, pub clip_held: Option<(usize, Vec<u8>)>, pub mix_scratch: Vec<u8> }
```

`held` is the last screen frame of the span before a mid-take display switch, with that span's index: what the switch's cross-dissolve blends FROM (`render::spans`). The loop latches it on the one output frame `FramePose::hold` names - copied while the decode buffer is still borrowed, stored after the composite releases it - and only uses it while `FramePose::mix` names that same span, so a cut that skipped the latch frame drops the dissolve instead of blending a stale picture. `None` all take long for a recording that never switched display, which costs that (overwhelmingly common) case nothing.

`clip_held` is the same idea one level up: the last screen frame of the OUTGOING clip, with that clip's index, latched by `latch_clip` at EVERY clip join, whether or not that join respawns anything. It is what a clip's `transition_in_ms` dissolve blends from, and the dissolve wants it at a forward join as much as at a backwards one, which is why the latch does not live inside `rewind`.

`mix_scratch` is the destination buffer the clip dissolve blends INTO, owned here so a dissolve allocates once for the whole export rather than once a frame. It is the clip-level twin of `FrameRenderer`'s own `mix_buf`, which the display-switch mix uses inside `composite_at`; the clip mix cannot share it, because it has to run BEFORE `composite_at` is called (the loop has to decide which slice to latch for `hold` and which to composite) and the two can be in flight on the same frame. It stays an empty `Vec` for the whole of any export without a clip dissolve.

The three decoders plus the held webcam frame and the webcam failure bookkeeping that `webcam_warning` reads after the loop. `advance` decodes ONE recording frame on the screen pipe (which then holds it) and the webcam pipe (holding its latest frame; a shorter webcam stream freezes on its last). `feed_bg` pulls one background-video frame per OUTPUT frame: the background runs on the output clock, so its frame index is the output frame index.

## Timing

```rust
pub(super) struct Timing { pub dec: u128, pub comp: u128, pub send: u128 }
```

Microseconds spent decoding, compositing and waiting on the encoder channel, for `exporter_report::log_timing`.

## Pipes::latch_clip

```rust
fn latch_clip(&mut self, prev_clip: usize)
```

Copies the screen pipe's currently held frame into `clip_held` under the outgoing clip's index. Called at EVERY clip join, before anything else happens at it, because a forward join replaces no decoder and would otherwise walk past the frame the dissolve needs without keeping it.

The latch is unconditional. It costs one screen-frame copy per join, a handful over a whole export, and the alternative is threading the clip list and its transitions down into the loop to find out whether this join happens to want one.

## Pipes::rewind

```rust
fn rewind(&mut self, d: &ClipDecode, clock: &Clock, first_k: u64) -> Result<()>
```

Throws the screen and webcam decoders away and starts them again at source frame `first_k`, seeking both to `first_k * 1000 / out_fps` (the seek rule, `mod.md`). **Called only where the plan goes BACKWARDS** (`ClipJoin::respawn`, `plan_walk.md`): a forward join is decoded through exactly as a cut is, by the same `decodes_needed = k - decoded` the cursor has always produced, so it needs no decoder of its own.

It spawns the replacement screen pipe FIRST (a failure returns before anything is swapped, so the old pipe is still the live one and the loop's error path can unwind cleanly), then swaps it in and `join`s the old one, which surfaces a screen decode error the loop never reached, and then hands the webcam to `rewind_webcam`.

The BACKGROUND pipe is deliberately NOT respawned. It runs on the OUTPUT clock, not the recording's: its frame index is the output frame index, so it is already at the right frame at a clip boundary, and restarting it would reset the wallpaper to its first frame every time a clip opened.

What a backwards join costs, measured rather than assumed: the screen seek is frame exact, but the webcam's is not. A 30 fps camera decoded at `-r 60` has each source frame duplicated across two output ticks, and `-ss` drops every frame before its seek point, so the duplication restarts on the first frame at or after it and the panel can run up to one webcam source frame (33 ms at 30 fps) AHEAD from that join to the end of the clip. That is the price of a reorder, and it is exactly why B4-R14 stopped paying it at forward joins.

## Pipes::rewind_webcam

```rust
fn rewind_webcam(&mut self, d: &ClipDecode, clock: &Clock, seek: u64)
```

The webcam half of a rewind, and deliberately softer than the screen half: it returns nothing, and every failure becomes a warning rather than an error, because the screen is the deliverable and the camera is one panel of it. A failed respawn, or a failed join of the old pipe, goes into `wc_fail` (first one wins, as everywhere else in this file) and the export finishes with a warning instead of dying. `wc_frames` keeps counting across the join, because the warning is about the whole export.

The ORDER is what makes a failure survivable. The new pipe is spawned before `last_webcam` is given up, and the held frame is only recycled into the old pipe's pool once the new pipe really exists. So a respawn that fails leaves `wpipe` as `None` with `last_webcam` still holding its last picture, and the camera panel FREEZES on that frame for the rest of the export, which is exactly what `webcam_warning` then tells the user. Recycling first would have handed the buffer away and left the panel absent instead, with a warning describing something else.

A pipe with no `d.webcam` path is not respawned at all (the same `exists` test the exporter's own first spawn makes), and freezes the same way.

**A SUCCESSFUL respawn past the end of the webcam track leaves the panel absent, not frozen.** `last_webcam` is taken as part of the swap and is only refilled by the next `advance`, and `WebcamPipe::next` answers `Ok(None)` at end of stream, so a backwards join that seeks past a camera track SHORTER than the screen recording has nothing to put back: the panel disappears from that join to the end of the export rather than holding its last picture, which is what `webcam_warning` describes and what the failed-respawn path above actually does. Narrow (it needs a webcam shorter than the take AND a reorder that lands in the tail), known, and not fixed here: keeping the previous frame when a respawn's first read is `None` is the change, and it belongs with whoever next opens the webcam path rather than inside a clips batch.

## Clock

```rust
pub(super) struct Clock { pub video_start: u64, pub out_fps: u64 }
```

## ClipDecode

```rust
pub(super) struct ClipDecode { pub video: PathBuf, pub webcam: Option<PathBuf>, pub screen_bytes: usize,
    pub screen_crop: Option<(u32, u32)>, pub wc_dims: (u32, u32), pub wc_bytes: usize, pub depth: usize,
    pub sw: u32, pub sh: u32 }
```

Everything a respawn needs that the loop does not otherwise carry: the two source files and the four decode shapes the exporter worked out once, before the first spawn. The loop gets one of these by reference for the whole export, because every clip decodes the same files at the same size; only the seek changes.

`sw`/`sh` are `RenderMeta`'s evened screen dims, and they are the one pair here a respawn does not use: the clip dissolve needs them, because `screen_mix::blend_into` walks an NV12 frame by hand and has to be told its planes' shape. They ride this struct rather than a parameter of their own because it is already the loop's one bag of decode shapes and they come from the same `meta` in the same breath.

`webcam` is an `Option` and is `None` when `paths.webcam()` does not exist, which is the same test the exporter's own first spawn makes: a recording with no camera must not acquire one at a clip boundary.

## run

```rust
pub(super) fn run(r: &mut FrameRenderer, pipes: &mut Pipes, plan: &[u64], spans: &[ClipSpan], dec: &ClipDecode,
                  video: &Path, clock: &Clock, out_dims: (u32, u32), out_pool: &BufPool,
                  tx: &SyncSender<Frame>, on_progress: &impl Fn(u8)) -> Result<(u64, Timing)>
```

Rides `FrameRenderer::walk_plan` (which steps the camera once per output frame, warms up before the first kept frame and snaps the cursor across a cut) and, inside its callback, advances the pipes by `PlanCursor::next().decodes_needed`, feeds the background, composites the held screen frame with the held webcam frame, sends the output frame with an OUTPUT-time timestamp (`video_start + j * 1000 / out_fps`), and reports progress as `j + 1` of the plan length. A decode error or a closed encoder channel stops the walk; the error, if any, is returned after it. Returns the frames actually sent and the timing split.

`spans` is `TimeMap::clip_spans(out_fps)` (`remap_spans.md`): where each clip's run of frames begins in the plan. It is why the loop can serve a plan that is not monotone, which a single forward-only decoder cannot. Before the cursor steps, `plan_walk::join_at(spans, plan, j)` asks whether this output frame opens a clip after the first. Every `Some` latches the outgoing frame with `Pipes::latch_clip`; only a `Some` whose `respawn` is set goes on to `Pipes::rewind` and then `PlanCursor::rebase(first_k)`, in that order, so the cursor counts the new decoder's first frame from the seek. A rewind failure is recorded like any other decode error and stops the walk.

**A forward join respawns nothing.** The decoders are left running and the cursor's ordinary `decodes_needed = k - decoded` walks them across the boundary, throwing away whatever lies between the outgoing clip's last frame and the incoming clip's first, which is precisely what it already does for a cut. So an in-order split costs what the unsplit take costs plus the frames it skips, and produces the same picture (ruling B4-R14, `plan_walk.md`). A long forward gap between two in-order clips therefore costs what a long cut costs today: it is decoded through.

**The dissolve.** After the screen frame is taken off the pipe and before anything is composited, the loop asks `pose.clip_mix` (`render::clipmix`) whether a clip transition is open on this frame. If it is, and `clip_held`'s stored clip index EQUALS the mix's `prev_clip`, the latched frame is blended into the decoded one through `screen_mix::blend_into` - the same NV12 primitive the display switch uses, with `alpha` as the incoming clip's weight - and the composite runs on `mix_scratch` instead. Comparing the indices is what makes a stale latch harmless: a latch from some earlier clip can never be blended, exactly as `held` is compared against `mix.prev_span` on the line below. `pose.scene.src` is passed as BOTH the source rect and the destination, which is an identity resample except where a clip boundary coincides with a mid-take display switch (a stated approximation, `clipmix.md`).

`to_hold` is taken from the picture AFTER that blend, on purpose: when a display switch wants this frame latched and a clip dissolve is in flight over it, the span mix must continue from the picture the viewer actually saw, not from the undissolved decode underneath it.

With no clips in the document there is exactly one span, `join_at` returns `None` at every index including 0, the cursor keeps `base == 0`, `pose.clip_mix` is `None` at every instant, the blend arm is never taken and `mix_scratch` never allocates: the loop runs the sequence it ran before clips existed, against the decoder the exporter spawned without a seek. A document WITH clips but no `transition_in_ms` is the same story one step further in, which is why `plain`, `split` and `split-off` all export frame for frame identically.
