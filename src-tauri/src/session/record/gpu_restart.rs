//! Moving a running GPU-native capture onto another display (or window) mid-take, without the
//! recorded file learning that it happened - the recorder half of `switch_display`.
//!
//! The encoder is the thing that must NOT restart: `video.mp4` is one H.264 stream at one size
//! with one PTS timeline, and the export decodes it against one `sync.json`. So a switch stops
//! the WGC capture, lifts the open `VideoEncoder`, the size it was built for and the recording
//! clock out of the dying `Cap`, and hands all three to a fresh capture on the new target as an
//! `EncoderSeed`. Frames from the new display are then simply a size change, which
//! `frame_scaler`/`frame_fit::letterbox` already fits into the canvas (aspect preserved, centred,
//! black bars) - the same path a browser tab toggling its bookmarks bar takes.
//!
//! Split from `gpu_record.rs` and `gpu_frames.rs` because both are at the 200-line cap.
use windows_capture::encoder::VideoEncoder;
use super::gpu_frames::{Cap, CapFlags, EncoderSpec, SizeHook};
use super::gpu_record::{start_capture, GpuRecorder, GpuStart};
use super::pause_clock::PauseClock;

/// The running encode, lifted out of one capture so the next one can continue it.
pub struct EncoderSeed {
    /// Still open, still holding `video.mp4`: nothing finalizes it across the switch.
    pub encoder: VideoEncoder,
    /// The size the encoder was built for - the take's canvas, which never changes again.
    pub dims: (u32, u32),
    /// The recording clock, carried so the PTS base, the monotonic check and `sync.json`'s
    /// timeline continue rather than restarting at zero on the new display's first frame.
    pub pause_clock: PauseClock,
}

impl Cap {
    /// Give up the encode: mark the handler as handed over (so its `on_closed` does not report
    /// the take as ended), and move the encoder, its size and the recording clock out. The
    /// `EncoderSpec` comes back too, because a switch made before the very first frame has no
    /// encoder yet and the replacement capture then has to build one itself.
    pub(super) fn take_seed(&mut self) -> (EncoderSpec, Option<EncoderSeed>) {
        self.handed_over = true;
        let (spec, dims) = (self.enc.clone(), self.enc_dims);
        // A handler must stay valid until its thread exits, so the real clock is swapped for a
        // fresh one rather than left moved-out.
        let pause_clock = std::mem::replace(&mut self.pause_clock, PauseClock::new(self.totals.clone()));
        (spec, self.encoder.take().map(|encoder| EncoderSeed { encoder, dims, pause_clock }))
    }
}

impl GpuRecorder {
    /// Stop this capture and start a new one on `target_id`, feeding the SAME encoder. Returns
    /// the replacement recorder and the new target's `(w, h)` - the capture's size, not the
    /// file's, which stays whatever the first frame of the take made it. `on_size` is told the
    /// replacement capture's first frame size (the true size; the returned pair is an estimate).
    ///
    /// Consumes `self`: on any failure the take has no capture left, and the caller
    /// (`VideoSink::switch`) is the one holding the frame timestamps that salvage it. The
    /// encoder is finalized on every one of those paths, by `finish()` here or by its own `Drop`
    /// in the thread that was handed it, so `video.mp4` is a playable file either way.
    pub fn restart(self, cfg: GpuStart, target_id: Option<&str>, on_size: SizeHook) -> anyhow::Result<(Self, u32, u32)> {
        let Self { control, frame_ts } = self;
        let cap = control.callback(); // Arc<Mutex<Cap>> - grab before stop() consumes control
        let (enc, seed) = cap.lock().take_seed(); // parking_lot Mutex: no poison
        // A WM_QUIT stop does not run `on_closed`, so nothing finalizes the MP4 behind us - and
        // the encoder is out of the handler's hands by now in any case.
        if let Err(e) = control.stop() {
            if let Some(s) = seed { let _ = s.encoder.finish(); }
            anyhow::bail!("gpu capture stop: {e:?}");
        }
        let flags = CapFlags {
            enc, clock: cfg.clock.clone(), frame_ts: frame_ts.clone(),
            paused: cfg.paused.clone(), totals: cfg.totals.clone(), ended: cfg.ended.clone(),
            seed, on_size,
        };
        let (control, w, h) = start_capture(&cfg, target_id, flags)?;
        Ok((Self { control, frame_ts }, w, h))
    }
}
