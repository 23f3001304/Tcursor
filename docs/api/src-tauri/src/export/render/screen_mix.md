# src-tauri/src/export/render/screen_mix.rs

The display-switch cross-dissolve, done on the DECODED SCREEN BUFFER rather than inside the compositors.

At a mid-take display switch the screen panel eases from the old display's aspect to the new one's while the two pictures cross-fade (`render::spans`). Both compositors already draw exactly one screen source cropped to exactly one rect, so the cheapest way to keep CPU and GPU - and therefore preview and export - on ONE code path is to hand them a single nv12 frame that is already the blend: the held frame's own span rect is resampled into the new span's rect and mixed in at `1 - alpha`. Drawing that one rect into the eased panel then squashes BOTH pictures by the same factor, which is what makes the morph read as one picture changing shape rather than two pictures sliding over each other.

*Why not a second screen input on `composite_into`:* it would mean two more nv12 textures, two more bind-group entries and a second sampling path in `shader.wgsl`, all of it duplicated in the CPU compositor and all of it live on every frame of every export to serve a 350 ms window. Blending the buffers instead leaves both compositors byte-identical to what they were and confines the whole feature to the frames that need it.

Everything happens in nv12, the format the decoders and both compositors already agree on, so no conversion is added to the frame path.

## blend_into

```rust
pub fn blend_into(out: &mut Vec<u8>, cur: &[u8], prev: &[u8], sw: u32, sh: u32,
                  prev_src: RectF, dst: RectF, alpha: f32)
```

Blend the held `prev` frame into `cur` over the destination rect, writing the result to `out` (nv12, `sw` x `sh`).

### Inputs

- `cur` / `prev` - the current decoded screen frame and the one latched before the switch (`FramePose::hold` named it; `SpanMix::hold_ms` says which output tick it is).
- `prev_src: RectF` - the rect the outgoing picture occupies in `prev` (the previous span's `src`).
- `dst: RectF` - the rect the incoming picture occupies in `cur` (the new span's `src`). *Why the destination is the NEW rect:* the compositor is given one crop rect, and it has to be the one the panel is shaped for.
- `alpha: f32` - the weight of `cur`: 0 at the switch instant, 1 at the end of the transition.

### Behaviors worth knowing

- `alpha >= 0.999` (and a `prev` of the wrong length) is a pure copy, so a caller always gets a complete frame back.
- Pixels OUTSIDE `dst` are untouched: the bars of the new span stay exactly as the decoder delivered them, so the crop still has clean edges to cut on.
- The chroma plane is blended at half resolution with both rects halved, so luma and chroma reach the midpoint together and a dissolve never drifts toward one source's colour.
- A ramp resampled from a wide source rect into a narrower destination is still a ramp: the held picture is SCALED into the new span's rect, not re-cropped out of it.

## half

```rust
fn half(r: RectF) -> RectF
```

A rect in half-resolution (chroma) coordinates.

## plane

```rust
fn plane(out: &mut [u8], prev: &[u8], w: u32, h: u32, prev_src: RectF, dst: RectF, a: f32, bpp: usize)
```

Blend one plane: for every pixel of `dst`, bilinearly sample `prev` at the matching point of `prev_src` and mix it under the destination pixel at `1 - a`. `bpp` is 1 (the Y plane) or 2 (the interleaved UV plane), which is the only difference between the two passes.

## sample

```rust
fn sample(p: &[u8], w: u32, h: u32, sx: f32, sy: f32, c: usize, bpp: usize) -> f32
```

Bilinear sample of channel `c` at (`sx`, `sy`), clamped to the plane's edges. Bilinear rather than nearest because the held picture is being rescaled by a non-integer factor and nearest would crawl visibly through the 350 ms it is on screen.
