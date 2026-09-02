//! 3-stage export pipeline: screen/webcam decode threads feed the composite loop (the
//! exporter), which feeds the encoder thread, connected by bounded channels + recycled
//! buffer pools so decode(N+1) overlaps composite(N) overlaps encode(N-1). Both the screen and
//! webcam decode at `-r out_fps` (see `ScreenPipe`/`WebcamPipe::spawn`), so they decode 1:1
//! with output frames at any export rate; the threads only move bytes.
use anyhow::{anyhow, Error, Result};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use crate::export::pipeline::pipeline_decode::{spawn_screen, spawn_webcam};
use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::ffio::RawDecoder;

/// INCLUSIVE frame-index bounds `[k_in, k_last]` for the trimmed export range, converting the
/// resolved trim range (ms, from `Trim::resolve`) to output-frame indices at `out_fps` - same
/// `(ms * out_fps) / 1000` formula the untrimmed loop already used for its own last index, so an
/// untrimmed clip (`trim_out_ms == full_dur_ms`) yields `k_last` identical to the old bound and
/// composites the exact same frame count. `k_last` is always `>= k_in`, so a degenerate
/// (zero-length) trim still emits at least one frame rather than producing an empty export.
pub fn trim_frame_bounds(trim_in_ms: u32, trim_out_ms: u32, out_fps: u64) -> (u64, u64) {
    let k_in = (trim_in_ms as u64 * out_fps) / 1000;
    let k_last = ((trim_out_ms as u64 * out_fps) / 1000).max(k_in);
    (k_in, k_last)
}

/// The mux shift (ms, negative = start later) for one audio track: its own start relative to
/// the exported video's frame 0. `trim_in_q_ms` MUST be the FRAME-FLOORED trim-in
/// (`k_in * 1000 / out_fps`, from `trim_frame_bounds`), NOT the raw `trim_in_ms`: exported frame
/// 0 shows the source content at that floored instant, so subtracting the unquantised ms value
/// advanced the audio ahead of the video by up to a full frame (33 ms at 30 fps).
pub fn audio_shift_ms(track_ms: Option<u64>, video_start: u64, trim_in_q_ms: u64) -> i64 {
    track_ms.map(|m| m as i64 - video_start as i64).unwrap_or(0) - trim_in_q_ms as i64
}

/// How the webcam decode ended, as a user-facing warning (`None` = nothing to say). The camera
/// is one panel of a deliverable whose picture is the screen, so none of these fail the export -
/// but each leaves a file that is not what was asked for, and all of them used to be silent.
/// `frames` (webcam frames actually composited) is what separates the two shapes of failure:
/// with none, the panel is ABSENT for the whole export; with some, it renders FROZEN on the last
/// decoded frame from that point on (`exporter`'s `last_webcam` hold), which looks like a stall
/// rather than a missing panel and needs saying differently.
pub fn webcam_warning(had_webcam: bool, fail: Option<&str>, frames: u64) -> Option<String> {
    match (had_webcam, fail, frames) {
        (false, _, _) => None,
        (_, Some(e), 0) => Some(format!("webcam decode failed before any frame, exported without the camera panel: {e}")),
        (_, Some(e), n) => Some(format!("webcam decode failed after {n} frames - the camera panel is frozen from that point on: {e}")),
        (_, None, 0) => Some("webcam decode produced no frames - exported without the camera panel".into()),
        (_, None, _) => None,
    }
}

/// Take (and clear) a stored decode-thread error, or `Ok` if none. Distinguishes a real
/// decode failure (surface it) from a clean EOF (channel closed, no error stored).
fn take_err(err: &Mutex<Option<Error>>) -> Result<()> {
    match err.lock().unwrap().take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Screen decode thread + main-side VFR advance. The thread streams `(buf, idx)` for
/// every decoded captured frame; `next_at` supersedes toward output time `t`, recycling
/// each passed-over buffer back to the decode pool.
pub struct ScreenPipe {
    rx: Receiver<(Vec<u8>, usize)>,
    returner: Sender<Vec<u8>>,
    cur: Option<(Vec<u8>, usize)>,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl ScreenPipe {
    /// Spawn the screen `RawDecoder` (spawn errors surface here) and its decode thread.
    /// `depth` sizes both the bounded channel and the recycled buffer pool. Decodes at
    /// `out_fps` (like `WebcamPipe::spawn`) so the decode rate-converts to the export's
    /// output rate instead of running at the source's native capture rate.
    pub fn spawn(video: &Path, screen_bytes: usize, target_dims: Option<(u32, u32)>, depth: usize, out_fps: u64) -> Result<ScreenPipe> {
        let dec = RawDecoder::spawn(video, out_fps as f64, false, None, None, target_dims, "nv12", screen_bytes)?;
        let pool = BufPool::new(depth, screen_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<(Vec<u8>, usize)>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_screen(dec, pool, tx, err.clone());
        Ok(ScreenPipe { rx, returner, cur: None, err, handle })
    }

    /// The next decoded screen frame - 1:1 with output frames. `video.mp4` is decoded at
    /// `-r out_fps` (see `spawn`), so output frame k IS decoded frame k at any export rate;
    /// re-timing against `sync.json` was wrong because the CFR encode has MORE frames than the
    /// recorded delivered-frame timestamps, skewing the video vs the real-time audio. On EOF
    /// holds the last decoded frame (matches the old clamp). `None` only for an empty video.
    /// Recycles the superseded buffer.
    pub fn next(&mut self) -> Result<Option<&[u8]>> {
        match self.rx.recv() {
            Ok(f) => { if let Some(old) = self.cur.replace(f) { let _ = self.returner.send(old.0); } }
            Err(_) => { take_err(&self.err)?; } // EOF: keep `cur` (hold the last frame)
        }
        Ok(self.cur.as_ref().map(|(b, _)| b.as_slice()))
    }

    /// Join the decode thread, surfacing a stored decode error. Drops the receiver first
    /// so a thread still trying to send (decoder outran the consumer) can't hang the join.
    pub fn join(self) -> Result<()> {
        let ScreenPipe { rx, handle, err, .. } = self;
        drop(rx);
        handle.join().map_err(|_| anyhow!("screen decode thread panicked"))?;
        take_err(&err)
    }
}

/// Webcam decode thread — one frame per output frame (1:1, no superseding). Simpler than
/// `ScreenPipe`; the caller recycles each buffer via `recycle` after compositing it.
pub struct WebcamPipe {
    rx: Receiver<Vec<u8>>,
    returner: Sender<Vec<u8>>,
    dims: (u32, u32),
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl WebcamPipe {
    /// Spawn the webcam `RawDecoder` (at `out_fps` - the export's resolved output frame rate,
    /// from `ExportSettings.fps` - seeked to `video_start`, cover-cropped to `dims`) and its
    /// decode thread. `depth` sizes the channel and the buffer pool. `dims` is the SOURCE-aspect
    /// decode box (`render::meta::webcam_box`) - ONE box for the whole export, which each panel
    /// cover-crops to its own aspect at composite time; it is not any panel's own `(w, h)`.
    pub fn spawn(webcam: &Path, video_start: u64, dims: (u32, u32), wc_bytes: usize, depth: usize, out_fps: u64) -> Result<WebcamPipe> {
        let dec = RawDecoder::spawn(webcam, out_fps as f64, false, Some(video_start), Some(dims), None, "bgra", wc_bytes)?;
        let pool = BufPool::new(depth, wc_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<Vec<u8>>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_webcam(dec, pool, tx, err.clone());
        Ok(WebcamPipe { rx, returner, dims, err, handle })
    }

    /// The next webcam frame `(buf, w, h)`, or `None` at EOF (matches the old
    /// `read_webcam` dropping the decoder so all later frames have no camera).
    pub fn next(&mut self) -> Result<Option<(Vec<u8>, u32, u32)>> {
        match self.rx.recv() {
            Ok(buf) => Ok(Some((buf, self.dims.0, self.dims.1))),
            Err(_) => { take_err(&self.err)?; Ok(None) }
        }
    }

    /// Return a composited webcam buffer to the pool for reuse.
    pub fn recycle(&self, buf: Vec<u8>) { let _ = self.returner.send(buf); }

    /// Join the decode thread, surfacing a stored decode error (drops rx first, like
    /// `ScreenPipe::join`, so an over-running webcam decoder can't hang the join).
    pub fn join(self) -> Result<()> {
        let WebcamPipe { rx, handle, err, .. } = self;
        drop(rx);
        handle.join().map_err(|_| anyhow!("webcam decode thread panicked"))?;
        take_err(&err)
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

pub mod pipeline_decode;
pub mod exporter;
pub mod run;
pub mod ffio;
pub mod audio_mux;
pub mod timeline;
