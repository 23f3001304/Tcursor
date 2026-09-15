use super::fit::{texture, RATE};
use crate::session::record::frame_fit::letterbox;
use anyhow::anyhow;
use wgc_windows::Win32::Foundation::RECT;
use wgc_windows::Win32::Graphics::Direct3D11::*;

pub struct Chain {
    pub(super) src: (u32, u32),
    pub(super) src_tex: ID3D11Texture2D,
    pub(super) processor: ID3D11VideoProcessor,
    pub(super) in_view: ID3D11VideoProcessorInputView,
    pub(super) out_view: ID3D11VideoProcessorOutputView,
}

impl Chain {
    pub fn new(
        video: &ID3D11VideoDevice,
        vctx: &ID3D11VideoContext,
        canvas: &ID3D11Texture2D,
        device: &ID3D11Device,
        dst: (u32, u32),
        src: (u32, u32),
    ) -> anyhow::Result<Self> {
        let desc = D3D11_VIDEO_PROCESSOR_CONTENT_DESC {
            InputFrameFormat: D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE,
            InputFrameRate: RATE,
            InputWidth: src.0,
            InputHeight: src.1,
            OutputFrameRate: RATE,
            OutputWidth: dst.0,
            OutputHeight: dst.1,
            Usage: D3D11_VIDEO_USAGE_PLAYBACK_NORMAL,
        };
        let (mut in_view, mut out_view) = (None, None);
        let input = D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC {
            FourCC: 0,
            ViewDimension: D3D11_VPIV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_INPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPIV {
                    MipSlice: 0,
                    ArraySlice: 0,
                },
            },
        };
        let output = D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC {
            ViewDimension: D3D11_VPOV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_VIDEO_PROCESSOR_OUTPUT_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_VPOV { MipSlice: 0 },
            },
        };
        let src_tex = texture(device, src)?;
        let chain = unsafe {
            let e = video.CreateVideoProcessorEnumerator(&desc)?;
            let processor = video.CreateVideoProcessor(&e, 0)?;
            video.CreateVideoProcessorInputView(&src_tex, &e, &input, Some(&mut in_view))?;
            video.CreateVideoProcessorOutputView(canvas, &e, &output, Some(&mut out_view))?;
            let (Some(in_view), Some(out_view)) = (in_view, out_view) else {
                return Err(anyhow!(
                    "the video processor gave back no input/output view"
                ));
            };
            Self {
                src,
                src_tex,
                processor,
                in_view,
                out_view,
            }
        };
        chain.configure(vctx, dst);
        Ok(chain)
    }

    fn configure(&self, vctx: &ID3D11VideoContext, dst: (u32, u32)) {
        let (l, t, r, b) = letterbox(self.src, dst);
        let rgba = D3D11_VIDEO_COLOR_RGBA {
            R: 0.0,
            G: 0.0,
            B: 0.0,
            A: 1.0,
        };
        let black = D3D11_VIDEO_COLOR {
            Anonymous: D3D11_VIDEO_COLOR_0 { RGBA: rgba },
        };
        let whole = RECT {
            left: 0,
            top: 0,
            right: self.src.0 as i32,
            bottom: self.src.1 as i32,
        };
        let fit = RECT {
            left: l,
            top: t,
            right: r,
            bottom: b,
        };
        let p = &self.processor;
        unsafe {
            vctx.VideoProcessorSetStreamFrameFormat(p, 0, D3D11_VIDEO_FRAME_FORMAT_PROGRESSIVE);
            vctx.VideoProcessorSetStreamAutoProcessingMode(p, 0, false);
            vctx.VideoProcessorSetStreamSourceRect(p, 0, true, Some(&whole));
            vctx.VideoProcessorSetStreamDestRect(p, 0, true, Some(&fit));
            vctx.VideoProcessorSetOutputTargetRect(p, false, None);
            vctx.VideoProcessorSetOutputBackgroundColor(p, false, &black);
        }
    }
}
