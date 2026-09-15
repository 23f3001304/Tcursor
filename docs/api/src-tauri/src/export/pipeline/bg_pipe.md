# src-tauri/src/export/pipeline/bg_pipe.rs

The VIDEO/GIF background's decode stream: a third ffmpeg process beside the screen and webcam ones, on the same output clock.

The whole design is **no timestamp math**. ffmpeg is told to loop the asset forever and emit it at the export's own frame rate, already scaled and cropped to cover the output size; the exporter then reads it strictly sequentially, so output frame `i` shows stream frame `i`. Nothing computes a presentation time, nothing seeks, and a re-run produces the same file byte for byte.

`WebcamPipe` (`pipeline/mod.rs`) is the template - one frame per output frame, no superseding - and `pipeline::spawn_webcam` is reused as the thread body for exactly that reason.

## bg_decode_args

```rust
pub fn bg_decode_args(asset: &Path, w: u32, h: u32, out_fps: u64) -> Vec<String>
```

The ffmpeg CLI for one looping background stream. Pure (no spawn), so the flag ORDER - the thing that makes the stream deterministic - is unit-testable without launching anything. That is also why this is not folded into `ffio_decoder::decode_args`: this shape needs input-side flags that function has no parameters for, and adding them would distort the three call sites that do not want them. `RawDecoder::spawn_args` is the shared half.

Flags, and why each one:

- `-stream_loop -1` **before `-i`** - an INPUT option (after `-i` it silently does nothing). A short clip is meant to repeat under a recording of any length, and looping in the demuxer costs nothing and never re-seeks.
- `-an` - a background is never a sound source, even when the file has audio.
- `-r <out_fps>` **after `-i`** - an OUTPUT rate. This is what makes "frame `i` of the stream" and "output frame `i`" the same thing: ffmpeg duplicates or drops source frames to hit it, once, up front. A 12fps GIF and a 60fps clip both arrive at the export's rate with no arithmetic on this side.
- `-vf scale=W:H:force_original_aspect_ratio=increase,crop=W:H` - cover the output, the same fit `ffio::decode_image_cover` gives a still, so a video background is framed like every other background.
- `-f rawvideo -pix_fmt bgra` - what the compositor uploads.

## BgPipe

```rust
pub struct BgPipe { /* rx, returner, dim, err, handle */ }
```

The decode thread plus the per-frame hand-off into the renderer's `bg` buffer. Holds `dim` because the dim is applied here, per frame, rather than by the compositor.

## BgPipe::open

```rust
pub fn open(asset: &Path, dims: (u32, u32), dim: f32, depth: usize, out_fps: u64) -> Result<BgPipe>
```

Spawn the looping decoder at the output size. `depth` sizes both the bounded channel and the recycled `BufPool`, exactly as it does for the other two streams, so the background decode overlaps the composite the same way.

An `Err` here is not fatal to an export: the caller logs it and falls back to the still first frame `background::build` already put in `bg`.

## BgPipe::next_frame

```rust
fn next_frame(&mut self) -> Option<Vec<u8>>
```

The next decoded frame, dimmed. The ONE point a frame leaves this pipe, so `dim` is applied exactly once no matter who asks - and through `background::apply_dim`, the same function the static buffer and the TS preview use, so all three agree on what 40% dim means.

## BgPipe::feed

```rust
pub fn feed(&mut self, r: &mut FrameRenderer) -> bool
```

Swap the next frame into `r`'s background buffer (`FrameRenderer::swap_bg`) and recycle the buffer it replaces back into the pool - a move each way, no ~8 MB copy.

`false` means the stream ended or failed. The caller keeps whatever `bg` already holds, which is why a dead stream shows a frozen background rather than a black one, and never fails an export.

## BgPipe::join

```rust
pub fn join(self) -> Result<()>
```

Join the decode thread, surfacing a stored decode error. Drops the receiver first so a thread still trying to send cannot hang the join (the same shape, and the same reason, as `WebcamPipe::join`).

## BgPipe::take

```rust
#[cfg(test)] pub fn take(&mut self) -> Option<Vec<u8>>
```

One frame's bytes, for the tests - the only caller that wants them rather than the renderer hand-off. Goes through `next_frame`, so what a test sees is what `feed` would swap in. (An earlier draft read the channel directly and made the dim test pass a frame the dim had never touched; it went through `next_frame` from then on.)

### Behaviors

- `the_stream_loops_forever_is_muted_and_covers_the_output` - `-stream_loop -1` precedes `-i`, `-r` follows it and carries the output fps, `-an` is present, and the filter and pixel format are exactly as documented above. Pure, always runs.
- `a_two_frame_clip_loops_so_frame_three_is_frame_one` - a 2-frame red/blue clip read three times gives back frame 1 again, at the cover-scaled output size. Needs a real ffmpeg (builds its fixture with `-f lavfi`, the way `ffmpeg_encoder`'s hardware probe builds its own); skipped, not failed, without one.
- `dim_is_applied_to_every_streamed_frame` - the same source frame opened at `dim 0.5` equals the undimmed one put through `apply_dim(0.5)`. Same ffmpeg guard.

The other half of "a video background actually reaches the screen" - that the GPU re-uploads it every frame instead of trusting a sampled content key - is `gpu_compositor::should_upload`, tested beside that key.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - opens one when `background::video_source` says the doc has a video background, and pulls a frame on every loop iteration.
- `src-tauri/src/export/render/bg.rs` (`FrameRenderer::swap_bg`) - where the frames land.
