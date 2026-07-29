# src-tauri/src/export/color.rs

nv12 <-> BGRA color conversion for the export screen path. The screen decoder emits **nv12** (Y plane + interleaved half-res UV, ~2.6x smaller than BGRA) so far fewer bytes cross the ffmpeg->exporter pipe (the measured export bottleneck) and ffmpeg skips its yuv->bgra convert. The GPU compositor converts nv12->RGB in the shader (`gpu/shader.wgsl` `screen_rgb`); this module is the CPU-side twin used by the fallback `CpuCompositor` and by tests.

Coefficients are **BT.601 limited-range**, which reproduces ffmpeg's default `-pix_fmt bgra` conversion for our "unknown" color-metadata captures - verified byte-exact against ffmpeg on a real 4K recording (`meanAbsDiff < 1`, `maxAbsDiff = 3`, pure rounding; see `color_tests::matches_ffmpeg_bgra`). The shader MUST use these exact constants so the GPU and CPU paths agree.

## yuv_to_rgb

```rust
pub fn yuv_to_rgb(y: f32, u: f32, v: f32) -> (u8, u8, u8)
```

One BT.601 limited-range YUV sample (0..255 each) to RGB (u8, clamped). Folded coefficients: `1.16438 = 255/219` on luma, chroma terms pre-scaled by `255/224`. The single source of truth for the color math, mirrored verbatim in the WGSL shader's `screen_rgb`.

## nv12_to_bgra

```rust
pub fn nv12_to_bgra(nv12: &[u8], w: u32, h: u32) -> Vec<u8>
```

Convert a full `w x h` nv12 buffer (`w*h*3/2` bytes) to BGRA (`w*h*4`). Y is `nv12[y*w + x]`; the covering UV pair is at `w*h + (y/2)*w + (x/2)*2`. Used by `CpuCompositor::composite_into` (the no-GPU fallback), which converts up front then runs its existing BGRA blend path.

## bgra_to_nv12

```rust
pub fn bgra_to_nv12(bgra: &[u8], w: u32, h: u32) -> Vec<u8>
```

Inverse of `nv12_to_bgra` (BT.601 limited-range), used only to build nv12 test inputs from the existing BGRA fixtures - the real pipeline gets nv12 straight from ffmpeg. Chroma is written once per 2x2 block (4:2:0), so it round-trips a solid color to within a few LSBs.
