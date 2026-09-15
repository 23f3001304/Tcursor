# src-tauri/src/ai/frames/extract.rs

WHAT the director sees at each moment `sample.rs` chose: one small JPEG per time, pulled out of the preview PROXY through ffmpeg. There is no JPEG encoder crate in `Cargo.toml`, so the encode is ffmpeg's own mjpeg muxer writing to stdout (A6); nothing is ever written to disk.

Sampling from the proxy rather than `video.mp4` is not an optimisation, it is the correctness rule (A5): the capture encoder writes CFR at a nominal rate faster than real capture, and `ensure_proxy_blocking` is the pass that stretches it back to the true duration. Proxy time is output time, which is the clock the transcript and every `EditOp` already use.

## LONG_EDGE

```rust
pub const LONG_EDGE: u32 = 512
```

Long side (px) of every frame the model sees. Enough for a vision model to read a dialog title, small enough that 16 of them do not blow a local model's context.

## JPEG_QV

```rust
pub const JPEG_QV: u32 = 5
```

ffmpeg's `-q:v` for the mjpeg encoder, whose scale is 2..31 with LOWER being better - NOT libjpeg's 0..100. 5 is about q80, the quality binding decision 1 asks for.

## jpeg_at

```rust
pub fn jpeg_at(video: &Path, t_ms: u32, long_edge: u32) -> Result<Vec<u8>, String>
```

One JPEG frame of `video` at `t_ms`, scaled to fit `long_edge` on its longer side.

### Inputs

- `video` - the file to seek in. Always a proxy in practice (see `jpegs_at`).
- `t_ms` - output-clock milliseconds.
- `long_edge` - the box the frame is fitted inside, aspect preserved.

### Returns

`Ok(bytes)` starting with the JPEG SOI marker `FF D8`. `Err` when ffmpeg could not be spawned, or when it produced no frame - which is what a seek past the last frame looks like. *Why an error rather than empty bytes:* a zero-byte "frame" handed to the model as perception would be a silent lie about what it saw.

### Implementation

`ffcmd_bg` (below-normal priority, so a propose pass cannot freeze the WebView), `-ss` BEFORE `-i` for a keyframe-accurate fast seek, `-frames:v 1`, `-vf scale=<e>:<e>:force_original_aspect_ratio=decrease`, `-f mjpeg -q:v JPEG_QV`, output to `-` (stdout). stderr is nulled: a seek past the end is an everyday case near the clip end, not something to log about.

## jpegs_at

```rust
pub fn jpegs_at(folder: &str, times: &[FrameAt], long_edge: u32) -> Vec<(FrameAt, Vec<u8>)>
```

Every frame of a project's proxy at `times`, each paired with the moment that asked for it.

### Implementation

Resolves the proxy ONCE with `ensure_proxy_blocking(folder, DEFAULT_PROXY_HEIGHT)` - which builds it if preprocessing never ran, so a project opened straight from disk still works - then calls `jpeg_at` per time and silently drops failures.

### Behaviors

- A proxy that cannot be built at all returns an empty list, never an error: the run continues text-only, which is the path that already works.
- The returned list may be shorter than `times`. `AiRun.frames` reports its length, not the number requested, so the sheet header never claims a frame the model did not receive.

## jpeg_dims

```rust
pub(crate) fn jpeg_dims(jpg: &[u8]) -> Option<(u32, u32)>
```

`(width, height)` read straight out of a JPEG's SOF marker, so the tests can check the resize actually happened without decoding anything.

### Implementation

Requires the `FF D8` SOI, then walks the marker chain from byte 2: skips fill bytes and the standalone `D0..D9`/`01` markers, and for any `FFC0..=FFCF` except `C4` (Huffman table), `C8` (JPEG extension) and `CC` (arithmetic coding conditioning) reads height at marker+5 and width at marker+7, both big-endian `u16`. Every other marker is skipped by its own length field. `None` for anything that is not a JPEG, including an empty slice.
