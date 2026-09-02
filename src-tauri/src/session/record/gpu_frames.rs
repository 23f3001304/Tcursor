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
use super::dim_guard::DimGuard;
use super::pause_clock::PauseClock;
use super::pause_totals::PauseTotals;
use super::{Notify, CAPTURE_CLOSED, DISPLAY_CHANGED};

/// Capture times (ms, pause-compressed) of the frames actually encoded, in encode order.
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;

/// Everything `Cap` needs, handed to it through `Settings`' flags slot.
pub struct CapFlags {
    pub encoder: VideoEncoder,
    pub clock: Arc<dyn Clock>,
    pub frame_ts: FrameTimes,
    pub paused: Arc<AtomicBool>,
    pub totals: Arc<PauseTotals>,
    pub ended: Notify,
    /// The `(w, h)` the encoder above was configured for, so `on_frame_arrived` can tell a
    /// mid-record dimension change apart from a normal frame - see `DimGuard`.
    pub dims: (u32, u32),
}

pub struct Cap {
    /// Taken by `GpuRecorder::stop` to finalize the MP4 (or by `on_closed`, whichever runs).
    pub encoder: Option<VideoEncoder>,
    clock: Arc<dyn Clock>,
    frame_ts: FrameTimes,
    paused: Arc<AtomicBool>,
    pause_clock: PauseClock,
    ended: Notify,
    /// The capture's D3D device + context, kept only so `on_frame_arrived` can rebuild the
    /// incoming frame around a rebased timestamp. Held as a `Context<()>` because the `windows`
    /// crate that names `ID3D11Device`/`ID3D11DeviceContext` here is windows-capture's own
    /// (0.61), a different version from this crate's (0.58) - so those two types cannot be
    /// named in a field declaration, while `Context<()>` can.
    gfx: Context<()>,
    /// The readback buffer `Frame::new` requires. Never touched: the rebuilt frame only ever
    /// reaches `send_frame`, which reads its surface and its timestamp and nothing else.
    scratch: Vec<u8>,
    /// Latches the FIRST frame whose size no longer matches the encoder (H1: a maximize/resize,
    /// display resolution/rotation change, or dock/undock mid-record).
    dims: DimGuard,
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
    encoded?;
    frame_ts.lock().unwrap_or_else(|e| e.into_inner()).push(sync_ms);
    Ok(())
}

impl GraphicsCaptureApiHandler for Cap {
    type Flags = CapFlags;
    type Error = anyhow::Error;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let Context { flags, device, device_context } = ctx;
        Ok(Self {
            encoder: Some(flags.encoder),
            clock: flags.clock,
            frame_ts: flags.frame_ts,
            paused: flags.paused,
            pause_clock: PauseClock::new(flags.totals),
            ended: flags.ended,
            gfx: Context { flags: (), device, device_context },
            scratch: Vec::new(),
            dims: DimGuard::new(flags.dims),
        })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, ctl: InternalCaptureControl) -> Result<(), Self::Error> {
        // Checked before anything else: the encoder was sized once, at start, and a mid-record
        // maximize/resize, display resolution/rotation change, or dock/undock hands us frames at
        // a new size from here on (finding H1 - the crate recreates its frame pool and keeps
        // delivering; it does not end the capture on its own). Finalize what's recorded so far
        // exactly like `on_closed` does for an OS-closed capture, then `ctl.stop()` - the SAME
        // internal-halt mechanism `GpuRecorder::stop`'s external `CaptureControl::stop` uses -
        // which also guarantees no later frame, mismatched or not, ever reaches this handler
        // again (the crate gates all future delivery on the halt flag `stop()` sets).
        if self.dims.mismatched((frame.width(), frame.height())) {
            let finished = self.encoder.take().map_or(Ok(()), |e| e.finish());
            (self.ended)(DISPLAY_CHANGED);
            ctl.stop();
            return Ok(finished?);
        }

        let now = self.clock.now_ms();
        let paused = self.paused.load(Ordering::SeqCst);
        let Some(tick) = self.pause_clock.tick(now, paused) else { return Ok(()) };

        let Self { encoder, gfx, scratch, frame_ts, .. } = self;
        // No encoder means `on_closed` already finalized the MP4 (the OS ended the capture);
        // this frame is not going into the file, so it must not go into sync.json either.
        let Some(enc) = encoder.as_mut() else { return Ok(()) };
        // Hand the encoder OUR clock instead of the frame's WGC `SystemRelativeTime`.
        // `send_frame` stamps the MF sample with `frame.timestamp() - first_timestamp` - the
        // raw QPC capture instant - which keeps every paused span alive in video.mp4's own PTS
        // while sync.json, the WAVs and all the input streams drop it; the export then decodes
        // that file 1:1 against sync.json's clock and gets a frozen span plus a pause-length
        // desync. Rebuilding the frame around the SAME GPU surface (no readback, no copy) with
        // `tick.pts_100ns` makes the file's timeline literally sync.json's timeline.
        let mut ts = frame.timestamp();
        ts.Duration = tick.pts_100ns;
        let mut rebased = Frame::new(
            &gfx.device,
            unsafe { frame.as_raw_surface().clone() },
            unsafe { frame.as_raw_texture().clone() },
            ts,
            &gfx.device_context,
            scratch,
            frame.width(),
            frame.height(),
            frame.color_format(),
            None,
        );
        record_if_encoded(frame_ts, tick.sync_ms, enc.send_frame(&mut rebased).map_err(Into::into))
    }

    /// Runs only when the OS closes the capture (the recorded window closed, the display went
    /// away), NOT on a WM_QUIT stop - `GpuRecorder::stop` finalizes that case. Finalize here so
    /// the MP4 is still valid, then tell the app: without this the video simply ends while the
    /// HUD keeps counting and the mic keeps recording narration over footage that stopped.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let finished = self.encoder.take().map_or(Ok(()), |e| e.finish());
        (self.ended)(CAPTURE_CLOSED);
        Ok(finished?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times() -> FrameTimes { Arc::new(Mutex::new(Vec::new())) }

    #[test]
    fn an_encoded_frame_gets_its_timestamp() {
        let ts = times();
        assert!(record_if_encoded(&ts, 1100, Ok(())).is_ok());
        assert_eq!(*ts.lock().unwrap(), vec![1100]);
    }

    /// A frame the encoder rejected is not in `video.mp4`, so it must not be in `sync.json`
    /// either - which since M1's salvage is written even when the take failed to finalize.
    #[test]
    fn a_failed_send_leaves_the_timestamps_untouched() {
        let ts = times();
        record_if_encoded(&ts, 100, Ok(())).unwrap();
        let err = record_if_encoded(&ts, 200, Err(anyhow::anyhow!("encoder gone")));
        assert!(err.is_err());
        assert_eq!(*ts.lock().unwrap(), vec![100], "an unencoded frame reached sync.json");
    }

    /// The send's error reaches the capture handler unchanged, so `CaptureControl::stop` still
    /// propagates it and the stop path still reports the take as failed.
    #[test]
    fn the_encode_error_is_propagated_verbatim() {
        let err = record_if_encoded(&times(), 0, Err(anyhow::anyhow!("mf sample rejected")));
        assert_eq!(err.unwrap_err().to_string(), "mf sample rejected");
    }
}
