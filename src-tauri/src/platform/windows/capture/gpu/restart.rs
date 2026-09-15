use super::frames::{Cap, CapFlags, EncoderSpec, SizeHook};
use super::record::{start_capture, GpuRecorder, GpuStart};
use crate::session::record::pause_clock::PauseClock;
use windows_capture::encoder::VideoEncoder;

pub struct EncoderSeed {
    pub encoder: VideoEncoder,
    pub dims: (u32, u32),
    pub pause_clock: PauseClock,
}

impl Cap {
    pub(super) fn take_seed(&mut self) -> (EncoderSpec, Option<EncoderSeed>) {
        self.handed_over = true;
        let (spec, dims) = (self.enc.clone(), self.enc_dims);
        let pause_clock =
            std::mem::replace(&mut self.pause_clock, PauseClock::new(self.totals.clone()));
        (
            spec,
            self.encoder.take().map(|encoder| EncoderSeed {
                encoder,
                dims,
                pause_clock,
            }),
        )
    }
}

impl GpuRecorder {
    pub fn restart(
        self,
        cfg: GpuStart,
        target_id: Option<&str>,
        on_size: SizeHook,
    ) -> anyhow::Result<(Self, u32, u32)> {
        let Self { control, frame_ts } = self;
        let cap = control.callback();
        let (enc, seed) = cap.lock().take_seed();
        if let Err(e) = control.stop() {
            if let Some(s) = seed {
                let _ = s.encoder.finish();
            }
            anyhow::bail!("gpu capture stop: {e:?}");
        }
        let flags = CapFlags {
            enc,
            clock: cfg.clock.clone(),
            frame_ts: frame_ts.clone(),
            paused: cfg.paused.clone(),
            totals: cfg.totals.clone(),
            ended: cfg.ended.clone(),
            seed,
            on_size,
        };
        let (control, w, h) = start_capture(&cfg, target_id, flags)?;
        Ok((Self { control, frame_ts }, w, h))
    }
}
