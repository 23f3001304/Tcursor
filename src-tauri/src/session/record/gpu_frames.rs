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
use super::pause_clock::PauseClock;
use super::pause_totals::PauseTotals;
use super::{Notify, CAPTURE_CLOSED};

/// Capture times (ms, pause-compressed) of the frames actually encoded, in encode order.
pub type FrameTimes = Arc<Mutex<Vec<u64>>>;

/// Everything `Cap` needs, handed to it through `Settings`' flags slot.
/// How to build the encoder ONCE the real capture size is known - see `Cap::encoder`.
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
    /// Built on the FIRST frame, from that frame's own size (see `on_frame_arrived`).
    enc: EncoderSpec,
    /// The size the encoder was actually built for (the first frame's). Set with it.
    enc_dims: (u32, u32),
    /// The fixed canvas every LATER size is scaled into (`frame_scaler.rs`). Costs nothing
    /// until the capture actually resizes: it builds itself on the first mismatched frame.
    fit: FrameFit,
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
            encoder: None,
            enc: flags.enc,
            enc_dims: (0, 0),
            clock: flags.clock,
            frame_ts: flags.frame_ts,
            paused: flags.paused,
            pause_clock: PauseClock::new(flags.totals),
            ended: flags.ended,
            gfx: Context { flags: (), device, device_context },
            scratch: Vec::new(),
            fit: FrameFit::new(),
        })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _ctl: InternalCaptureControl) -> Result<(), Self::Error> {
        // The encoder is built HERE, from the first frame's own size - never from
        // `GetWindowRect`, which on Win10/11 includes invisible DWM resize margins the capture
        // surface does not have. Sizing it from that inflated rect meant every single frame
        // mismatched the encoder: dropping them emptied the sink ("no samples were processed"),
        // and encoding them anyway made it read each row at the wrong stride, writing magenta/
        // green video. Taking the size from the surface makes the common case exact, so the fit
        // below is only ever needed for a LATER change.
        if self.encoder.is_none() {
            self.enc_dims = (frame.width(), frame.height());
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
            match fit.fit(gfx, frame, dims) { Some(pair) => Some(pair), None => return Ok(()) }
        };
        // Hand the encoder OUR clock instead of the frame's WGC `SystemRelativeTime`.
        // `send_frame` stamps the MF sample with `frame.timestamp() - first_timestamp` - the
        // raw QPC capture instant - which keeps every paused span alive in video.mp4's own PTS
        // while sync.json, the WAVs and all the input streams drop it; the export then decodes
        // that file 1:1 against sync.json's clock and gets a frozen span plus a pause-length
        // desync. Rebuilding the frame around the SAME GPU surface (no readback, no copy) with
        // `tick.pts_100ns` makes the file's timeline literally sync.json's timeline.
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
