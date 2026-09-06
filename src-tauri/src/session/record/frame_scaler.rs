//! The OBS-style fit itself: ONE persistent D3D11 canvas at the encoder's size, with the D3D11
//! video processor scaling every differently-sized capture frame into it. A mid-record resize
//! (a browser tab switch toggling the bookmarks bar, a window resize, a swapchain recreation)
//! therefore neither ends the take nor freezes the picture - it just gets fitted.
//!
//! Split from `gpu_frames.rs` (the WGC callback, already at the line cap) the way `pause_clock`
//! and `close_guard` were, and from `frame_fit.rs` because the geometry there is pure and
//! unit-tested while nothing here can run without a live GPU capture. The D3D11 types are
//! windows-capture's own `windows` 0.61, not this crate's 0.58 - see `Cargo.toml`'s
//! `wgc-windows` entry for why that is one crate instance and not a second dependency.
use anyhow::anyhow;
use std::mem::ManuallyDrop;
use std::slice;
use wgc_windows::core::Interface;
use wgc_windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use wgc_windows::Win32::Graphics::Direct3D11::*;
use wgc_windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_RATIONAL, DXGI_SAMPLE_DESC};
use wgc_windows::Win32::Graphics::Dxgi::IDXGISurface;
use wgc_windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11SurfaceFromDXGISurface;
use windows_capture::capture::Context;
use windows_capture::frame::Frame;
use super::frame_chain::Chain;

/// Input and output are the same stream, so the processor never converts frame rates; the value
/// only has to be non-zero, which some drivers insist on.
pub(super) const RATE: DXGI_RATIONAL = DXGI_RATIONAL { Numerator: 60, Denominator: 1 };

/// `IDirect3DSurface` is the one interface here the `windows` crate does not mark `Send`, and
/// `Cap` (which owns this) is moved onto the crate's capture thread. It is agile in practice -
/// windows-capture's own encoder hands the very same interface to its transcode thread through
/// a private `SendDirectX` wrapper - so this mirrors that rather than inventing a claim.
struct SendSurface(IDirect3DSurface);
unsafe impl Send for SendSurface {}

/// One BGRA render-target texture at `dims`, on the capture's own device.
pub(super) fn texture(device: &ID3D11Device, dims: (u32, u32)) -> anyhow::Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: dims.0,
        Height: dims.1,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut tex = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex))? };
    tex.ok_or_else(|| anyhow!("CreateTexture2D returned no texture"))
}

/// The canvas plus the device-level video objects, built once on the first resized frame.
struct Scaler {
    dst: (u32, u32),
    canvas: ID3D11Texture2D,
    surface: SendSurface,
    video: ID3D11VideoDevice,
    vctx: ID3D11VideoContext,
    chain: Option<Chain>,
    /// A source size whose processor chain could not be built. Latched so a driver that refuses
    /// one particular size is asked ONCE, not once per frame: without it a persistent resize
    /// (a window left at the new size) retries `CreateVideoProcessor*` at capture frame rate for
    /// the rest of the take - strictly more work per skipped frame than the plain skip it falls
    /// back to anyway.
    bad_src: Option<(u32, u32)>,
}

impl Scaler {
    fn new(gfx: &Context<()>, dst: (u32, u32)) -> anyhow::Result<Self> {
        let canvas = texture(&gfx.device, dst)?;
        // Derived exactly the way windows-capture derives the surface it hands the encoder: the
        // texture's `IDXGISurface`, wrapped by `CreateDirect3D11SurfaceFromDXGISurface`. Made
        // once, with the canvas, because it is the canvas - not a per-frame view of it.
        let dxgi: IDXGISurface = canvas.cast()?;
        let surface: IDirect3DSurface =
            unsafe { CreateDirect3D11SurfaceFromDXGISurface(&dxgi)? }.cast()?;
        let (video, vctx) = (gfx.device.cast()?, gfx.device_context.cast()?);
        Ok(Self { dst, canvas, surface: SendSurface(surface), video, vctx, chain: None, bad_src: None })
    }

    fn blit(&mut self, gfx: &Context<()>, src_tex: &ID3D11Texture2D, src: (u32, u32)) -> anyhow::Result<()> {
        if self.bad_src == Some(src) {
            anyhow::bail!("no video-processor chain for {src:?} (latched)");
        }
        if !matches!(&self.chain, Some(c) if c.src == src) {
            match Chain::new(&self.video, &self.vctx, &self.canvas, &gfx.device, self.dst, src) {
                Ok(chain) => self.chain = Some(chain),
                Err(e) => { self.bad_src = Some(src); return Err(e); }
            }
        }
        let chain = self.chain.as_ref().expect("just built");
        // `Default` zeroes it: one enabled stream, no past/future surfaces, no stereo right eye.
        // The clone is an AddRef, not an allocation, and is released again below - the field is
        // a `ManuallyDrop`, so letting `stream` fall out of scope would leak the view.
        let mut stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            pInputSurface: ManuallyDrop::new(Some(chain.in_view.clone())),
            ..Default::default()
        };
        // The frame's own texture belongs to WGC's frame pool and is recycled under us, so it
        // cannot be the thing the input view points at. Copying it into the chain's fixed-size
        // input texture instead keeps that ONE view valid for every frame, which is what makes
        // the hot path allocation-free.
        let blt = unsafe {
            gfx.device_context.CopyResource(&chain.src_tex, src_tex);
            self.vctx.VideoProcessorBlt(&chain.processor, &chain.out_view, 0, slice::from_ref(&stream))
        };
        unsafe { ManuallyDrop::drop(&mut stream.pInputSurface) };
        Ok(blt?)
    }
}

/// The lazily-built scaler as `Cap` holds it, plus the two states that mean "do not try": no
/// resize has happened yet, and setup failed once (a device with no video processor, say).
/// Latching the failure keeps the hot path from rebuilding a failing chain sixty times a second
/// and leaves the old behaviour - skip the frame, keep recording - as the floor.
pub struct FrameFit(Fit);

enum Fit { Idle, Ready(Scaler), Unavailable }

impl FrameFit {
    pub const fn new() -> Self { Self(Fit::Idle) }

    /// Scale `frame` into the fixed `dst` canvas and return that canvas' surface and texture for
    /// `Frame::new`. `None` means the frame could not be fitted and must be skipped - never a
    /// wrongly sized surface, which the encoder would read at the wrong stride and write as
    /// magenta/green video.
    pub fn fit(&mut self, gfx: &Context<()>, frame: &Frame, dst: (u32, u32)) -> Option<(IDirect3DSurface, ID3D11Texture2D)> {
        if matches!(self.0, Fit::Idle) {
            self.0 = Scaler::new(gfx, dst).map_or(Fit::Unavailable, Fit::Ready);
        }
        let Fit::Ready(s) = &mut self.0 else { return None };
        s.blit(gfx, unsafe { frame.as_raw_texture() }, (frame.width(), frame.height())).ok()?;
        Some((s.surface.0.clone(), s.canvas.clone()))
    }
}
