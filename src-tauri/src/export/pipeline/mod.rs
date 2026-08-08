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
    rx: Receiver<(Vec<u8>, u32)>,
    returner: Sender<Vec<u8>>,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl WebcamPipe {
    /// Spawn the webcam `RawDecoder` (at `out_fps` - the export's resolved output frame rate,
    /// from `ExportSettings.fps` - seeked to `video_start`, cover-cropped to `size`) and its
    /// decode thread. `depth` sizes the channel and the buffer pool.
    pub fn spawn(webcam: &Path, video_start: u64, size: u32, wc_bytes: usize, depth: usize, out_fps: u64) -> Result<WebcamPipe> {
        let dec = RawDecoder::spawn(webcam, out_fps as f64, false, Some(video_start), Some(size), None, "bgra", wc_bytes)?;
        let pool = BufPool::new(depth, wc_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<(Vec<u8>, u32)>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_webcam(dec, pool, tx, size, err.clone());
        Ok(WebcamPipe { rx, returner, err, handle })
    }

    /// The next webcam frame `(buf, w, h)`, or `None` at EOF (matches the old
    /// `read_webcam` dropping the decoder so all later frames have no camera).
    pub fn next(&mut self) -> Result<Option<(Vec<u8>, u32, u32)>> {
        match self.rx.recv() {
            Ok((buf, sz)) => Ok(Some((buf, sz, sz))),
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
mod tests {
    use super::trim_frame_bounds;
    #[test]
    fn trim_frame_bounds_converts_ms_to_inclusive_frame_indices() {
        assert_eq!(trim_frame_bounds(0, 10_000, 60), (0, 600));
        assert_eq!(trim_frame_bounds(2_000, 8_000, 60), (120, 480));
    }

    #[test]
    fn trim_frame_bounds_never_collapses_to_empty() {
        let (k_in, k_last) = trim_frame_bounds(5_000, 5_000, 60);
        assert!(k_last >= k_in);
    }

    /// Back-compat: an untrimmed clip (`trim_out_ms == full_dur_ms`, the seeded default) must
    /// yield the exact same last frame index the old `total_out = dur*fps/1000` bound computed,
    /// so the export loop's frame count is unchanged when nothing is actually trimmed.
    #[test]
    fn untrimmed_last_index_matches_the_old_total_out_formula() {
        let full_dur_ms = 12_345u32;
        let old_total_out = (full_dur_ms as u64 * 60) / 1000;
        let (k_in, k_last) = trim_frame_bounds(0, full_dur_ms, 60);
        assert_eq!((k_in, k_last), (0, old_total_out));
    }
}

pub mod pipeline_decode;
pub mod exporter;
pub mod run;
pub mod ffio;
pub mod audio_mux;
pub mod timeline;
