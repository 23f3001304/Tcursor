# src-tauri/src/platform/windows/capture/gpu/chain.rs

The OBS-style fit itself: ONE persistent D3D11 canvas at the encoder's size, with the D3D11 video processor scaling every differently-sized capture frame into it. A mid-record resize - a browser tab switch toggling the bookmarks bar, a window resize, a swapchain recreation - therefore neither ends the take nor freezes the picture.

Split from `gpu/frames.rs` (the WGC callback, already at the 200-line cap) the way `pause_clock.rs` and `close_guard.rs` were, and from `frame_fit.rs` because the geometry there is pure and unit-tested while nothing here can run without a live GPU capture.

*Why the `wgc_windows` crate alias:* the D3D11 device, context and texture a WGC frame carries are windows-capture's `windows` 0.61 types, not this crate's 0.58 ones, and two versions of the same interface are distinct Rust types. `Cargo.toml` therefore names the same 0.61 package a second time as `wgc-windows`, with a subset of the features windows-capture already enables - cargo unifies both entries onto the one crate instance, so nothing extra is downloaded, compiled or linked, and the types here are literally the frame's own. The alternative was transmuting COM pointers between crate versions.

## Chain

```rust
struct Chain {
    src: (u32, u32),
    src_tex: ID3D11Texture2D,
    processor: ID3D11VideoProcessor,
    in_view: ID3D11VideoProcessorInputView,
    out_view: ID3D11VideoProcessorOutputView,
}
```

The video-processor chain for ONE source size. `D3D11_VIDEO_PROCESSOR_CONTENT_DESC` bakes the input size in, so a capture that changes size *again* rebuilds this - but never the canvas, which is what the encoder was configured for and has to outlive every resize.

- `src_tex` - a fixed-size copy target for the frame. *Why not just view the frame's own texture:* that texture belongs to WGC's frame pool and is recycled under us, so an input view built on it would be valid for one frame only, and rebuilding one per frame would allocate on the recording hot path. Copying into `src_tex` keeps the single `in_view` valid for every frame.

## Chain::new

```rust
fn new(s: &Scaler, device: &ID3D11Device, src: (u32, u32)) -> anyhow::Result<Self>
```

Builds the enumerator (input `src`, output the scaler's canvas size, progressive, `PLAYBACK_NORMAL`, a non-zero and identical in/out frame rate because some drivers reject `0/0`), then the processor, then the two texture views, then calls `configure`. The enumerator itself is not kept: it is only needed to create the processor and the views.

## Chain::configure

```rust
fn configure(&self, vctx: &ID3D11VideoContext, dst: (u32, u32))
```

Everything the blit needs that cannot change for this source size, set ONCE so the per-frame path is two GPU calls and no state:

- the fit - stream source rect = the whole frame, stream dest rect = `frame_fit::letterbox(src, dst)`;
- `VideoProcessorSetOutputTargetRect(.., false, None)` so the target is the whole canvas;
- `VideoProcessorSetOutputBackgroundColor(.., black)`. *This is the letterbox.* The processor repaints the entire target rect with the background on every blit before compositing the stream, so a previous, larger frame can never bleed through the bars - no separate clear pass is needed, or possible to forget.
- `VideoProcessorSetStreamAutoProcessingMode(.., false)`. Drivers default it ON, which lets them slip denoise/sharpen/frame-rate tricks into a recording that is meant to be a faithful master for the export to re-composite.
