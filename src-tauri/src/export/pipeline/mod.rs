//! 3-stage export pipeline: screen/webcam decode threads feed the composite loop (the
//! exporter), which feeds the encoder thread, connected by bounded channels + recycled
//! buffer pools so decode(N+1) overlaps composite(N) overlaps encode(N-1). The pixel
//! selection rule is the pure `advance_index`; the threads only move bytes, never change
//! them, so output stays byte-identical to the old sequential loop.
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

/// Index of the captured frame active at output time `t`: the last `i >= cur` with
/// `frames[i] <= t`, clamped to the last frame. Mirrors the exporter's VFR advance
/// (`while frames[i+1] <= t`). Assumes `frames` is non-decreasing (capture timestamps).
pub fn advance_index(frames: &[u64], cur: usize, t: u64) -> usize {
    let mut i = cur;
    while i + 1 < frames.len() && frames[i + 1] <= t {
        i += 1;
    }
    i
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
    frames: Vec<u64>,
    cur: Option<(Vec<u8>, usize)>,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl ScreenPipe {
    /// Spawn the screen `RawDecoder` (spawn errors surface here) and its decode thread.
    /// `depth` sizes both the bounded channel and the recycled buffer pool.
    pub fn spawn(video: &Path, screen_bytes: usize, target_dims: Option<(u32, u32)>, frames: Vec<u64>, depth: usize) -> Result<ScreenPipe> {
        let dec = RawDecoder::spawn(video, 0.0, false, None, None, target_dims, screen_bytes)?;
        let pool = BufPool::new(depth, screen_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<(Vec<u8>, usize)>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_screen(dec, pool, tx, err.clone());
        Ok(ScreenPipe { rx, returner, frames, cur: None, err, handle })
    }

    /// The captured screen frame active at output time `t`, blocking on the decode
    /// channel as needed. `None` only for a zero-frame video. On EOF, clamps at the last
    /// decoded frame (the old `have == false` behavior). Recycles superseded buffers.
    pub fn next_at(&mut self, t: u64) -> Result<Option<&[u8]>> {
        if self.cur.is_none() {
            match self.rx.recv() {
                Ok(f) => self.cur = Some(f),
                Err(_) => { take_err(&self.err)?; return Ok(None); }
            }
        }
        let target = advance_index(&self.frames, self.cur.as_ref().unwrap().1, t);
        while self.cur.as_ref().unwrap().1 < target {
            match self.rx.recv() {
                Ok(next) => {
                    let old = self.cur.replace(next).unwrap().0;
                    let _ = self.returner.send(old); // recycle the superseded buffer
                }
                Err(_) => { take_err(&self.err)?; break; } // EOF short of target: clamp at cur
            }
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
        let dec = RawDecoder::spawn(webcam, out_fps as f64, false, Some(video_start), Some(size), None, wc_bytes)?;
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
    use super::{advance_index, trim_frame_bounds};
    #[test]
    fn advance_index_matches_sequential_selection() {
        let frames = vec![0u64, 100, 250, 400];
        assert_eq!(advance_index(&frames, 0, 0), 0);
        assert_eq!(advance_index(&frames, 0, 99), 0);
        assert_eq!(advance_index(&frames, 0, 100), 1);
        assert_eq!(advance_index(&frames, 1, 300), 2);
        assert_eq!(advance_index(&frames, 2, 10_000), 3); // clamps at last
        assert_eq!(advance_index(&frames, 0, 10_000), 3); // never goes backwards, clamps
    }

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
