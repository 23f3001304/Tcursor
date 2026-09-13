// The VIDEO/GIF background's decode stream: a third ffmpeg process beside the screen and webcam
// ones, on the same output clock.
//
// The whole design is "no timestamp math": ffmpeg is told to loop the asset forever and emit it at
// the export's own frame rate, already scaled and cropped to cover the output size. The exporter
// then reads it strictly sequentially, so output frame i shows stream frame i. Nothing computes a
// presentation time, nothing seeks, and a re-run produces the same file byte for byte.
//
// `WebcamPipe` (pipeline/mod.rs) is the template - one frame per output frame, no superseding -
// and `spawn_webcam` is reused as the thread body for exactly that reason.
use anyhow::{anyhow, Error, Result};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::pipeline::pipeline_decode::spawn_webcam;
use crate::export::render::FrameRenderer;
use crate::export::scene::background::apply_dim;

/// The ffmpeg CLI for one looping background stream. Pure (no spawn), so the flag order that makes
/// this deterministic is unit-testable without launching anything.
pub fn bg_decode_args(asset: &Path, w: u32, h: u32, out_fps: u64) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error"].map(String::from).into();
    // INPUT options, before -i: loop the file forever. A short clip is meant to repeat under a
    // recording of any length, and looping in the demuxer costs nothing and never re-seeks.
    args.push("-stream_loop".into()); args.push("-1".into());
    args.push("-i".into()); args.push(asset.to_string_lossy().into_owned());
    args.push("-an".into()); // a background is never a sound source, even when the file has audio
    // OUTPUT rate: the export's own. This is what makes "frame i of the stream" and "output frame
    // i" the same thing - ffmpeg duplicates or drops source frames to hit it, once, up front.
    args.push("-r".into()); args.push(format!("{:.4}", out_fps as f64));
    // Cover the output: scale up until it fills, then centre-crop - the same fit
    // `ffio::decode_image_cover` gives a still, so a video background is framed like every other.
    args.push("-vf".into());
    args.push(format!("scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"));
    args.push("-f".into()); args.push("rawvideo".into());
    args.push("-pix_fmt".into()); args.push("bgra".into()); // what the compositor uploads
    args.push("-".into());
    args
}

/// The background decode thread + the per-frame hand-off into the renderer's `bg` buffer.
pub struct BgPipe {
    rx: Receiver<Vec<u8>>,
    returner: Sender<Vec<u8>>,
    dim: f32,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl BgPipe {
    /// Spawn the looping decoder for `asset` at the output size. `depth` sizes both the bounded
    /// channel and the recycled buffer pool, exactly as it does for the other two streams.
    pub fn open(asset: &Path, dims: (u32, u32), dim: f32, depth: usize, out_fps: u64) -> Result<BgPipe> {
        let bytes = (dims.0 as usize) * (dims.1 as usize) * 4;
        let dec = RawDecoder::spawn_args(bg_decode_args(asset, dims.0, dims.1, out_fps), bytes)?;
        let pool = BufPool::new(depth, bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<Vec<u8>>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_webcam(dec, pool, tx, err.clone());
        Ok(BgPipe { rx, returner, dim, err, handle })
    }

    /// The next decoded frame, dimmed. The ONE point a frame leaves this pipe, so `dim` is applied
    /// exactly once no matter who asks - the same guarantee `background::build` gives the static
    /// buffer, with the same function.
    fn next_frame(&mut self) -> Option<Vec<u8>> {
        let mut buf = self.rx.recv().ok()?;
        apply_dim(&mut buf, self.dim);
        Some(buf)
    }

    /// Swap the next decoded frame into `r`'s background buffer, recycling the one it replaces.
    /// `false` means the stream ended or failed - the caller keeps whatever `bg` already holds
    /// (the asset's first frame, built by `background::build`), which is why a dead stream shows a
    /// frozen background rather than a black one and never fails an export.
    pub fn feed(&mut self, r: &mut FrameRenderer) -> bool {
        let Some(buf) = self.next_frame() else { return false };
        let old = r.swap_bg(buf);
        let _ = self.returner.send(old);
        true
    }

    /// Join the decode thread, surfacing a stored decode error. Drops the receiver first so a
    /// thread still trying to send can't hang the join (same shape as `WebcamPipe::join`).
    pub fn join(self) -> Result<()> {
        let BgPipe { rx, handle, err, .. } = self;
        drop(rx);
        handle.join().map_err(|_| anyhow!("background decode thread panicked"))?;
        let stored = err.lock().unwrap().take();
        match stored { Some(e) => Err(e), None => Ok(()) }
    }

    /// One frame's bytes, for the tests - the only caller that wants them rather than the
    /// renderer hand-off. Goes through `next_frame`, so what it sees is what `feed` would swap in.
    #[cfg(test)]
    pub fn take(&mut self) -> Option<Vec<u8>> { self.next_frame() }
}

#[cfg(test)]
#[path = "bg_pipe_tests.rs"]
mod tests;
