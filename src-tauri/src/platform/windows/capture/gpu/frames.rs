use super::fit::FrameFit;
use super::restart::EncoderSeed;
use crate::domain::time::Clock;
use crate::session::record::pause_clock::PauseClock;
use crate::session::record::pause_totals::PauseTotals;
use crate::session::record::{Notify, CAPTURE_CLOSED};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::VideoEncoder;
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;

pub type FrameTimes = Arc<Mutex<Vec<u64>>>;

pub type SizeHook = Option<Box<dyn FnOnce(u32, u32) + Send>>;

#[derive(Clone)]
pub struct EncoderSpec {
    pub fps: u32,
    pub path: String,
}

pub struct CapFlags {
    pub enc: EncoderSpec,
    pub clock: Arc<dyn Clock>,
    pub frame_ts: FrameTimes,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    pub(super) seed: Option<EncoderSeed>,
    pub(super) on_size: SizeHook,
}

pub struct Cap {
    pub encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
    pub(super) pause_clock: PauseClock,
    pub(super) totals: Arc<PauseTotals>,
    ended: Notify,
    pub(super) handed_over: bool,
    gfx: Context<()>,
    scratch: Vec<u8>,
    pub(super) enc: EncoderSpec,
    pub(super) enc_dims: (u32, u32),
    fit: FrameFit,
    on_size: SizeHook,
}

fn record_if_encoded(
    frame_ts: &FrameTimes,
    sync_ms: u64,
    encoded: anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Err(e) = &encoded {
        eprintln!("[capture] encoder rejected a frame at {sync_ms} ms: {e:#}");
    }
    encoded?;
    frame_ts
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(sync_ms);
    Ok(())
}

impl GraphicsCaptureApiHandler for Cap {
    type Flags = CapFlags;
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let Context {
            flags,
            device,
            device_context,
        } = ctx;
        let (encoder, enc_dims, pause_clock) = match flags.seed {
            Some(s) => (Some(s.encoder), s.dims, s.pause_clock),
            None => (None, (0, 0), PauseClock::new(flags.totals.clone())),
        };
        Ok(Self {
            encoder,
            enc: flags.enc,
            enc_dims,
            clock: flags.clock,
            frame_ts: flags.frame_ts,
            paused: flags.paused,
            pause_clock,
            totals: flags.totals,
            ended: flags.ended,
            handed_over: false,
            gfx: Context {
                flags: (),
                device,
                device_context,
            },
            scratch: Vec::new(),
            fit: FrameFit::new(),
            on_size: flags.on_size,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _ctl: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if let Some(f) = self.on_size.take() {
            f(frame.width(), frame.height());
        }
        if self.encoder.is_none() {
            self.enc_dims = ((frame.width() & !1).max(2), (frame.height() & !1).max(2));
            self.encoder = Some(super::record::encoder(
                self.enc_dims.0,
                self.enc_dims.1,
                self.enc.fps,
                &self.enc.path,
            )?);
        }

        let now = self.clock.now_ms();
        let paused = self.paused.load(Ordering::SeqCst);
        let Some(tick) = self.pause_clock.tick(now, paused) else {
            return Ok(());
        };

        let Self {
            encoder,
            gfx,
            scratch,
            frame_ts,
            fit,
            enc_dims,
            ..
        } = self;
        let Some(enc) = encoder.as_mut() else {
            return Ok(());
        };
        let dims = *enc_dims;
        let fitted = if (frame.width(), frame.height()) == dims {
            None
        } else {
            match fit.fit(gfx, frame, dims) {
                Some(pair) => Some(pair),
                None => {
                    eprintln!(
                        "[capture] no fit for a {}x{} frame into {}x{}",
                        frame.width(),
                        frame.height(),
                        dims.0,
                        dims.1
                    );
                    return Ok(());
                }
            }
        };
        let mut ts = frame.timestamp();
        ts.Duration = tick.pts_100ns;
        let (surface, texture) = fitted.unwrap_or_else(|| unsafe {
            (
                frame.as_raw_surface().clone(),
                frame.as_raw_texture().clone(),
            )
        });
        let mut rebased = Frame::new(
            &gfx.device,
            surface,
            texture,
            ts,
            &gfx.device_context,
            scratch,
            dims.0,
            dims.1,
            frame.color_format(),
            None,
        );
        record_if_encoded(
            frame_ts,
            tick.sync_ms,
            enc.send_frame(&mut rebased).map_err(Into::into),
        )
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let finished = self.encoder.take().map_or(Ok(()), |e| e.finish());
        if !self.handed_over {
            (self.ended)(CAPTURE_CLOSED);
        }
        Ok(finished?)
    }
}

#[cfg(test)]
#[path = "frames_tests.rs"]
mod tests;
