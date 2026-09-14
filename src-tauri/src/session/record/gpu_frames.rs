//! The WGC frame callback for the GPU-native path: hardware-encode each arriving frame on the
//! GPU (no readback) with a pause-aware PTS, and record its capture time for `sync.json`. Split
//! from `gpu_record.rs` (which owns the encoder settings and the recorder lifecycle) so both
//! stay under the line cap.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::VideoEncoder;
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use crate::domain::time::Clock;
use super::frame_scaler::FrameFit;
use super::gpu_restart::EncoderSeed;
use super::pause_clock::PauseClock;
use super::pause_totals::PauseTotals;
use super::{Notify, CAPTURE_CLOSED};

/// Capture times (ms, pause-compressed) of the frames actually encoded, in encode order.
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;

/// Told the raw size of a capture's FIRST frame, once: `switch_display` uses it to write the
/// switched-to target's true pixel size into its `DisplaySwitch` (the estimate from its window
/// rect is off by the invisible DWM borders, and the render crops by this size to the pixel).
pub type SizeHook = Option<Box<dyn FnOnce(u32, u32) + Send>>;

/// Everything `Cap` needs, handed to it through `Settings`' flags slot.
/// How to build the encoder ONCE the real capture size is known - see `Cap::encoder`.
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
    /// `Some` only for the replacement capture a mid-take display switch starts
    /// (`gpu_restart.rs`): the encoder, its size and the recording clock of the capture this one
    /// takes over from, so `video.mp4` keeps one encoder and one timeline across the switch.
    pub(super) seed: Option<EncoderSeed>,
    pub(super) on_size: SizeHook,
}

pub struct Cap {
    /// Taken by `GpuRecorder::stop` to finalize the MP4 (or by `on_closed`, whichever runs),
    /// and by `take_seed` to hand it to the replacement capture of a display switch.
    pub encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
    pub(super) pause_clock: PauseClock,
    /// The ledger `pause_clock` reads, kept beside it so `take_seed` can move the real clock out
    /// and leave a fresh one behind (a handler must stay valid until its thread exits).
    pub(super) totals: Arc<PauseTotals>,
    ended: Notify,
    /// Set by `take_seed`: the encoder now belongs to the replacement capture, so `on_closed`
    /// must not report the take as ended.
    pub(super) handed_over: bool,
    /// The capture's D3D device + context, for rebuilding a frame around a rebased timestamp. A
    /// `Context<()>` because its device types come from windows-capture's own `windows` (0.61), not this crate's (0.58).
    gfx: Context<()>,
    /// The readback buffer `Frame::new` requires. Never touched: the rebuilt frame only ever
    /// reaches `send_frame`, which reads its surface and its timestamp and nothing else.
    scratch: Vec<u8>,
    /// Built on the FIRST frame, from that frame's own size (see `on_frame_arrived`).
    pub(super) enc: EncoderSpec,
    /// The size the encoder was actually built for (the first frame's). Set with it.
    pub(super) enc_dims: (u32, u32),
    /// The fixed canvas every LATER size is scaled into (`frame_scaler.rs`). Costs nothing
    /// until the capture actually resizes: it builds itself on the first mismatched frame.
    fit: FrameFit,
    /// Taken and called on the first frame; `None` from then on.
    on_size: SizeHook,
}

/// Append `sync_ms` to `frame_ts` only once the frame it describes is actually IN the file:
/// `encoded` is the send's own result, evaluated by the caller before this runs, and its error
/// propagates with `frame_ts` untouched.
///
/// The ordering is the guarantee. It used to be the other way round, which was free while a
/// send error also meant `sync.json` was never written - M1's salvage ended that, so a
/// pre-push would now leave a trailing timestamp describing a frame the file does not contain,
/// which is exactly the frame<->timestamp desync this whole task exists to remove.
fn record_if_encoded(frame_ts: &FrameTimes, sync_ms: u64, encoded: anyhow::Result<()>) -> anyhow::Result<()> {
    if let Err(e) = &encoded { eprintln!("[capture] encoder rejected a frame at {sync_ms} ms: {e:#}"); }
    encoded?;
    frame_ts.lock().unwrap_or_else(|e| e.into_inner()).push(sync_ms);
    Ok(())
}

impl GraphicsCaptureApiHandler for Cap {
    type Flags = CapFlags;
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let Context { flags, device, device_context } = ctx;
        // A seeded capture replaces one a display switch stopped: it inherits that capture's open
        // encoder, the size it was built for and its `PauseClock`, so the PTS base and
        // `sync.json`'s clock carry over and no second encoder is ever built. `FrameFit` is NOT
        // inherited - it allocates on the capture's own D3D device, and this is a new one.
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
            gfx: Context { flags: (), device, device_context },
            scratch: Vec::new(),
            fit: FrameFit::new(),
            on_size: flags.on_size,
        })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _ctl: InternalCaptureControl) -> Result<(), Self::Error> {
        // The first frame's raw size, told once: the switch that started this capture records it.
        if let Some(f) = self.on_size.take() { f(frame.width(), frame.height()); }
        // The encoder is built HERE, from the first frame's own size - never from `GetWindowRect`,
        // which includes invisible DWM margins the surface lacks (sized from that, every frame
        // mismatched: dropped = an empty sink, encoded = the wrong stride, magenta/green video).
        // A seeded capture (a display switch) has the take's encoder and `enc_dims` already, so
        // this is skipped and the new display is fitted below, like a window resize.
        if self.encoder.is_none() {
            // Rounded DOWN to even: H.264 4:2:0 has no odd sizes, and an odd-sized window capture
            // came out as an odd-sized file whose nv12 frames no longer measured `w*h*3/2` bytes,
            // so the export sheared (2026-09-14). The fit scales such a frame in with no bars.
            self.enc_dims = ((frame.width() & !1).max(2), (frame.height() & !1).max(2));
            self.encoder = Some(super::gpu_record::encoder(
                self.enc_dims.0, self.enc_dims.1, self.enc.fps, &self.enc.path)?);
        }

        let now = self.clock.now_ms();
        let paused = self.paused.load(Ordering::SeqCst);
        let Some(tick) = self.pause_clock.tick(now, paused) else { return Ok(()) };

        let Self { encoder, gfx, scratch, frame_ts, fit, enc_dims, .. } = self;
        // No encoder means `on_closed` already finalized the MP4 (the OS ended the capture);
        // this frame is not going into the file, so it must not go into sync.json either.
        let Some(enc) = encoder.as_mut() else { return Ok(()) };
        // A LATER size change - a browser tab switch toggling the bookmarks bar, a window resize,
        // a swapchain recreation - cannot go into this fixed-size MP4 at its own size: the
        // encoder would read every row at the wrong stride and write magenta/green. Fit it into a
        // persistent canvas at `enc_dims` instead (`frame_scaler.rs`: aspect preserved, centred,
        // black bars, one video-processor blit on the GPU) and encode THAT, so a resize neither
        // ends the take nor freezes the picture on the last good frame. Only if the fit itself
        // fails is the frame skipped, and `record_if_encoded` then withholds its timestamp too,
        // so `sync.json` still never describes a frame the file lacks.
        let dims = *enc_dims;
        let fitted = if (frame.width(), frame.height()) == dims { None } else {
            match fit.fit(gfx, frame, dims) { Some(pair) => Some(pair), None => {
                eprintln!("[capture] no fit for a {}x{} frame into {}x{}", frame.width(), frame.height(), dims.0, dims.1);
                return Ok(());
            } }
        };
        // Hand the encoder OUR clock, not the frame's WGC `SystemRelativeTime`: that raw capture
        // instant keeps every paused span alive in video.mp4's PTS while sync.json and every
        // stream drop it. Rebuilt around the SAME surface (no copy), the file's timeline IS sync.json's.
        let mut ts = frame.timestamp();
        ts.Duration = tick.pts_100ns;
        // The fitted canvas when the capture resized, the frame's own surface when it did not.
        let (surface, texture) = fitted.unwrap_or_else(|| unsafe {
            (frame.as_raw_surface().clone(), frame.as_raw_texture().clone())
        });
        let mut rebased = Frame::new(
            &gfx.device, surface, texture, ts, &gfx.device_context, scratch,
            dims.0, dims.1, frame.color_format(), None,
        );
        record_if_encoded(frame_ts, tick.sync_ms, enc.send_frame(&mut rebased).map_err(Into::into))
    }

    /// Runs only when the OS closes the capture (the recorded window closed, the display went
    /// away), NOT on a WM_QUIT stop - `GpuRecorder::stop` finalizes that case. Finalize here so
    /// the MP4 is still valid, then tell the app: without this the video simply ends while the
    /// HUD keeps counting and the mic keeps recording narration over footage that stopped.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let finished = self.encoder.take().map_or(Ok(()), |e| e.finish());
        // After `take_seed` this handler is only waiting for its thread to exit: reporting the
        // take as ended would stop a recording still running on the other display.
        if !self.handed_over { (self.ended)(CAPTURE_CLOSED); }
        Ok(finished?)
    }
}

#[cfg(test)]
#[path = "gpu_frames_tests.rs"]
mod tests;
