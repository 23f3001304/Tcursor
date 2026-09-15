use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::pipeline::spawn_webcam;
use crate::export::render::FrameRenderer;
use crate::export::scene::background::apply_dim;
use anyhow::{anyhow, Error, Result};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub fn bg_decode_args(asset: &Path, w: u32, h: u32, out_fps: u64) -> Vec<String> {
    let mut args: Vec<String> = ["-v", "error"].map(String::from).into();
    args.push("-stream_loop".into());
    args.push("-1".into());
    args.push("-i".into());
    args.push(asset.to_string_lossy().into_owned());
    args.push("-an".into());
    args.push("-r".into());
    args.push(format!("{:.4}", out_fps as f64));
    args.push("-vf".into());
    args.push(format!(
        "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
    ));
    args.push("-f".into());
    args.push("rawvideo".into());
    args.push("-pix_fmt".into());
    args.push("bgra".into());
    args.push("-".into());
    args
}

pub struct BgPipe {
    rx: Receiver<Vec<u8>>,
    returner: Sender<Vec<u8>>,
    dim: f32,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl BgPipe {
    pub fn open(
        asset: &Path,
        dims: (u32, u32),
        dim: f32,
        depth: usize,
        out_fps: u64,
    ) -> Result<BgPipe> {
        let bytes = (dims.0 as usize) * (dims.1 as usize) * 4;
        let dec = RawDecoder::spawn_args(bg_decode_args(asset, dims.0, dims.1, out_fps), bytes)?;
        let pool = BufPool::new(depth, bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<Vec<u8>>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_webcam(dec, pool, tx, err.clone());
        Ok(BgPipe {
            rx,
            returner,
            dim,
            err,
            handle,
        })
    }

    fn next_frame(&mut self) -> Option<Vec<u8>> {
        let mut buf = self.rx.recv().ok()?;
        apply_dim(&mut buf, self.dim);
        Some(buf)
    }

    pub fn feed(&mut self, r: &mut FrameRenderer) -> bool {
        let Some(buf) = self.next_frame() else {
            return false;
        };
        let old = r.swap_bg(buf);
        let _ = self.returner.send(old);
        true
    }

    pub fn join(self) -> Result<()> {
        let BgPipe {
            rx, handle, err, ..
        } = self;
        drop(rx);
        handle
            .join()
            .map_err(|_| anyhow!("background decode thread panicked"))?;
        let stored = err.lock().unwrap().take();
        match stored {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    pub fn take(&mut self) -> Option<Vec<u8>> {
        self.next_frame()
    }
}

#[cfg(test)]
#[path = "bg_pipe_tests.rs"]
mod tests;
