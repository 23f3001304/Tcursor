# src-tauri/src/platform/windows/capture/gpu/fit.rs

The OBS-style fit itself: ONE persistent D3D11 canvas at the encoder's size, with the D3D11 video processor scaling every differently-sized capture frame into it. A mid-record resize - a browser tab switch toggling the bookmarks bar, a window resize, a swapchain recreation - therefore neither ends the take nor freezes the picture.

Split from `gpu/frames.rs` (the WGC callback, already at the 200-line cap) the way `pause_clock.rs` and `close_guard.rs` were, and from `frame_fit.rs` because the geometry there is pure and unit-tested while nothing here can run without a live GPU capture.

*Why the `wgc_windows` crate alias:* the D3D11 device, context and texture a WGC frame carries are windows-capture's `windows` 0.61 types, not this crate's 0.58 ones, and two versions of the same interface are distinct Rust types. `Cargo.toml` therefore names the same 0.61 package a second time as `wgc-windows`, with a subset of the features windows-capture already enables - cargo unifies both entries onto the one crate instance, so nothing extra is downloaded, compiled or linked, and the types here are literally the frame's own. The alternative was transmuting COM pointers between crate versions.

## SendSurface

```rust
struct SendSurface(IDirect3DSurface);
```

`IDirect3DSurface` is the one interface used here that the `windows` crate does not mark `Send`, and `Cap` (which owns the `FrameFit` that owns this) is moved onto windows-capture's own capture thread. The interface is agile in practice - the crate's own `VideoEncoder` hands the very same type to its transcode thread through a private `SendDirectX` wrapper - so this mirrors that rather than inventing a new claim.

## texture

```rust
fn texture(device: &ID3D11Device, dims: (u32, u32)) -> anyhow::Result<ID3D11Texture2D>
```

One BGRA8 `D3D11_USAGE_DEFAULT` texture at `dims` on the capture's own device, bound as both a render target and a shader resource - the flags the video processor needs to use it as an output view and as an input view respectively. Used for the canvas and for each chain's input staging texture.

## Scaler

```rust
struct Scaler {
    dst: (u32, u32),
    canvas: ID3D11Texture2D,
    surface: SendSurface,
    video: ID3D11VideoDevice,
    vctx: ID3D11VideoContext,
    chain: Option<Chain>,
    bad_src: Option<(u32, u32)>,
}
```

The canvas plus the device-level video objects, built once on the first resized frame and reused for the rest of the take. `chain` is `None` until the first blit and is replaced only when the source size changes again; `bad_src` remembers a source size whose chain could not be built, so it is never retried.

## Scaler::new

```rust
fn new(gfx: &Context<()>, dst: (u32, u32)) -> anyhow::Result<Self>
```

Allocates the canvas at `dst` and derives its `IDirect3DSurface` exactly the way the vendored windows-capture crate does - cast the texture to `IDXGISurface`, wrap it with `CreateDirect3D11SurfaceFromDXGISurface`, cast the resulting `IInspectable` - then casts the capture's device and device context to their video interfaces. The surface is made ONCE, with the canvas, because it *is* the canvas rather than a per-frame view of it.

## Scaler::blit

```rust
fn blit(&mut self, gfx: &Context<()>, src_tex: &ID3D11Texture2D, src: (u32, u32)) -> anyhow::Result<()>
```

Rebuilds `chain` if `src` changed, then per frame: `CopyResource` the frame's texture into `chain.src_tex`, and `VideoProcessorBlt` that into the canvas through the pre-configured processor. No CPU readback, no heap allocation - the `D3D11_VIDEO_PROCESSOR_STREAM` is a zeroed stack struct and the input view it carries is a `ManuallyDrop` clone, i.e. one AddRef, explicitly released again after the blit (letting it fall out of scope would leak the view, since `ManuallyDrop` never drops).

## FrameFit

```rust
pub struct FrameFit(Fit);
```

## Fit

```rust
enum Fit { Idle, Ready(Scaler), Unavailable }
```

The lazily-built scaler as `Cap` holds it, plus the two states that mean "do not try": `Idle` (no resize has happened yet - a recording that never resizes pays nothing for this module) and `Unavailable` (setup failed once, e.g. a device with no usable video processor). Latching the failure keeps the hot path from rebuilding a failing chain sixty times a second and leaves the old behaviour - skip the frame, keep recording - as the floor.

## FrameFit::new

```rust
pub const fn new() -> Self
```

`Idle`. Nothing is allocated until a frame actually arrives at the wrong size.

## FrameFit::fit

```rust
pub fn fit(&mut self, gfx: &Context<()>, frame: &Frame, dst: (u32, u32)) -> Option<(IDirect3DSurface, ID3D11Texture2D)>
```

Scale `frame` into the fixed `dst` canvas and return that canvas' surface and texture, which `Cap::on_frame_arrived` hands to `Frame::new` in place of the frame's own pair.

`None` means the frame could not be fitted and must be skipped - never a wrongly sized surface. *Why that matters:* the MP4's sink writer is configured for `dst`, so a surface of any other size makes the encoder read every row at the wrong stride and write magenta/green video. Skipping is the same fallback the code had before this module existed, and `record_if_encoded` then withholds the frame's timestamp too, so `sync.json` still cannot describe a frame the file lacks.

A failure inside `Scaler::new` latches `Unavailable` for the whole take. A failure to build the *chain* latches only that one source size, in `bad_src`: a driver that refuses one particular size is asked once rather than once per frame - without it a persistent resize (a window simply left at its new size) would retry `CreateVideoProcessor*` at capture frame rate for the rest of the recording, which is strictly more work per skipped frame than the plain skip it falls back to anyway. A different size is still tried normally.
