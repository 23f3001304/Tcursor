# src-tauri/src/export/preview/encode.rs

Encoding a composited preview frame for the wire: PNG (lossless, for the background and the frame bench), JPEG (what the stage actually shows), and the base64 the data URLs need. Pure output formatting - no rendering, no decoding - split out of `preview/mod.rs` for its line budget so that file stays the render path alone. Re-exported from `preview::mod`, so every existing `crate::export::preview::png_encode` path is unchanged.

## jpeg_encode

```rust
pub(crate) fn jpeg_encode(bgra: &[u8], w: u32, h: u32) -> Result<Vec<u8>>
```

JPEG-encodes a BGRA buffer through ffmpeg (`-f rawvideo -pix_fmt bgra` from a `StagedInput` temp file, `-frames:v 1 -q:v 3 -f mjpeg -`). *Why not `png_encode`:* the stage draws this frame the moment playback pauses or a scrub settles, and a 1280-wide PNG deflate in a debug build costs more than the render itself; ffmpeg's encoder is native code at any build profile and the result is a tenth of the bytes over IPC. Errors when ffmpeg fails or returns fewer than four bytes. Test: `jpeg_tests::a_bgra_buffer_encodes_to_a_jpeg` (skips without ffmpeg).

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
