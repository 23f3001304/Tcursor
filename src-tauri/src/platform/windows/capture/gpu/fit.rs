use super::chain::Chain;
use anyhow::anyhow;
use std::mem::ManuallyDrop;
use std::slice;
use wgc_windows::core::Interface;
use wgc_windows::Graphics::DirectX::Direct3D11::IDirect3DSurface;
use wgc_windows::Win32::Graphics::Direct3D11::*;
use wgc_windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use wgc_windows::Win32::Graphics::Dxgi::IDXGISurface;
use wgc_windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11SurfaceFromDXGISurface;
use windows_capture::capture::Context;
use windows_capture::frame::Frame;

pub(super) const RATE: DXGI_RATIONAL = DXGI_RATIONAL {
    Numerator: 60,
    Denominator: 1,
};

struct SendSurface(IDirect3DSurface);
unsafe impl Send for SendSurface {}

pub(super) fn texture(device: &ID3D11Device, dims: (u32, u32)) -> anyhow::Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: dims.0,
        Height: dims.1,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut tex = None;
    unsafe { device.CreateTexture2D(&desc, None, Some(&mut tex))? };
    tex.ok_or_else(|| anyhow!("CreateTexture2D returned no texture"))
}

struct Scaler {
    dst: (u32, u32),
    canvas: ID3D11Texture2D,
    surface: SendSurface,
    video: ID3D11VideoDevice,
    vctx: ID3D11VideoContext,
    chain: Option<Chain>,
    bad_src: Option<(u32, u32)>,
}

impl Scaler {
    fn new(gfx: &Context<()>, dst: (u32, u32)) -> anyhow::Result<Self> {
        let canvas = texture(&gfx.device, dst)?;
        let dxgi: IDXGISurface = canvas.cast()?;
        let surface: IDirect3DSurface =
            unsafe { CreateDirect3D11SurfaceFromDXGISurface(&dxgi)? }.cast()?;
        let (video, vctx) = (gfx.device.cast()?, gfx.device_context.cast()?);
        Ok(Self {
            dst,
            canvas,
            surface: SendSurface(surface),
            video,
            vctx,
            chain: None,
            bad_src: None,
        })
    }

    fn blit(
        &mut self,
        gfx: &Context<()>,
        src_tex: &ID3D11Texture2D,
        src: (u32, u32),
    ) -> anyhow::Result<()> {
        if self.bad_src == Some(src) {
            anyhow::bail!("no video-processor chain for {src:?} (latched)");
        }
        if !matches!(&self.chain, Some(c) if c.src == src) {
            match Chain::new(
                &self.video,
                &self.vctx,
                &self.canvas,
                &gfx.device,
                self.dst,
                src,
            ) {
                Ok(chain) => self.chain = Some(chain),
                Err(e) => {
                    self.bad_src = Some(src);
                    return Err(e);
                }
            }
        }
        let chain = self.chain.as_ref().expect("just built");
        let mut stream = D3D11_VIDEO_PROCESSOR_STREAM {
            Enable: true.into(),
            pInputSurface: ManuallyDrop::new(Some(chain.in_view.clone())),
            ..Default::default()
        };
        let blt = unsafe {
            gfx.device_context.CopyResource(&chain.src_tex, src_tex);
            self.vctx.VideoProcessorBlt(
                &chain.processor,
                &chain.out_view,
                0,
                slice::from_ref(&stream),
            )
        };
        unsafe { ManuallyDrop::drop(&mut stream.pInputSurface) };
        Ok(blt?)
    }
}

pub struct FrameFit(Fit);

enum Fit {
    Idle,
    Ready(Scaler),
    Unavailable,
}

impl FrameFit {
    pub const fn new() -> Self {
        Self(Fit::Idle)
    }

    pub fn fit(
        &mut self,
        gfx: &Context<()>,
        frame: &Frame,
        dst: (u32, u32),
    ) -> Option<(IDirect3DSurface, ID3D11Texture2D)> {
        if matches!(self.0, Fit::Idle) {
            self.0 = Scaler::new(gfx, dst).map_or(Fit::Unavailable, Fit::Ready);
        }
        let Fit::Ready(s) = &mut self.0 else {
            return None;
        };
        s.blit(
            gfx,
            unsafe { frame.as_raw_texture() },
            (frame.width(), frame.height()),
        )
        .ok()?;
        Some((s.surface.0.clone(), s.canvas.clone()))
    }
}
