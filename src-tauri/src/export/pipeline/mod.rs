use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::ffio::RawDecoder;
use anyhow::{anyhow, Error, Result};
use std::path::Path;
use std::sync::mpsc::{sync_channel, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub fn trim_frame_bounds(trim_in_ms: u32, trim_out_ms: u32, out_fps: u64) -> (u64, u64) {
    let k_in = (trim_in_ms as u64 * out_fps) / 1000;
    let k_last = ((trim_out_ms as u64 * out_fps) / 1000).max(k_in);
    (k_in, k_last)
}

pub fn audio_shift_ms(track_ms: Option<u64>, video_start: u64, trim_in_q_ms: u64) -> i64 {
    track_ms.map(|m| m as i64 - video_start as i64).unwrap_or(0) - trim_in_q_ms as i64
}

pub fn webcam_warning(had_webcam: bool, fail: Option<&str>, frames: u64) -> Option<String> {
    match (had_webcam, fail, frames) {
        (false, _, _) => None,
        (_, Some(e), 0) => Some(format!("webcam decode failed before any frame, exported without the camera panel: {e}")),
        (_, Some(e), n) => Some(format!("webcam decode failed after {n} frames - the camera panel is frozen from that point on: {e}")),
        (_, None, 0) => Some("webcam decode produced no frames - exported without the camera panel".into()),
        (_, None, _) => None,
    }
}

fn take_err(err: &Mutex<Option<Error>>) -> Result<()> {
    match err.lock().unwrap().take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub struct ScreenPipe {
    rx: Receiver<(Vec<u8>, usize)>,
    returner: Sender<Vec<u8>>,
    cur: Option<(Vec<u8>, usize)>,
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl ScreenPipe {
    pub fn spawn(
        video: &Path,
        screen_bytes: usize,
        crop: Option<(u32, u32)>,
        target_dims: Option<(u32, u32)>,
        depth: usize,
        out_fps: u64,
    ) -> Result<ScreenPipe> {
        let dec = RawDecoder::spawn(
            video,
            out_fps as f64,
            false,
            None,
            crop,
            None,
            target_dims,
            "nv12",
            screen_bytes,
        )?;
        let pool = BufPool::new(depth, screen_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<(Vec<u8>, usize)>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_screen(dec, pool, tx, err.clone());
        Ok(ScreenPipe {
            rx,
            returner,
            cur: None,
            err,
            handle,
        })
    }

    pub fn next(&mut self) -> Result<Option<&[u8]>> {
        match self.rx.recv() {
            Ok(f) => {
                if let Some(old) = self.cur.replace(f) {
                    let _ = self.returner.send(old.0);
                }
            }
            Err(_) => {
                take_err(&self.err)?;
            }
        }
        Ok(self.cur.as_ref().map(|(b, _)| b.as_slice()))
    }

    pub fn held(&self) -> Option<&[u8]> {
        self.cur.as_ref().map(|(b, _)| b.as_slice())
    }

    pub fn join(self) -> Result<()> {
        let ScreenPipe {
            rx, handle, err, ..
        } = self;
        drop(rx);
        handle
            .join()
            .map_err(|_| anyhow!("screen decode thread panicked"))?;
        take_err(&err)
    }
}

pub struct WebcamPipe {
    rx: Receiver<Vec<u8>>,
    returner: Sender<Vec<u8>>,
    dims: (u32, u32),
    err: Arc<Mutex<Option<Error>>>,
    handle: JoinHandle<()>,
}

impl WebcamPipe {
    pub fn spawn(
        webcam: &Path,
        video_start: u64,
        dims: (u32, u32),
        wc_bytes: usize,
        depth: usize,
        out_fps: u64,
    ) -> Result<WebcamPipe> {
        let dec = RawDecoder::spawn(
            webcam,
            out_fps as f64,
            false,
            Some(video_start),
            None,
            Some(dims),
            None,
            "bgra",
            wc_bytes,
        )?;
        let pool = BufPool::new(depth, wc_bytes);
        let returner = pool.returner();
        let (tx, rx) = sync_channel::<Vec<u8>>(depth);
        let err = Arc::new(Mutex::new(None));
        let handle = spawn_webcam(dec, pool, tx, err.clone());
        Ok(WebcamPipe {
            rx,
            returner,
            dims,
            err,
            handle,
        })
    }

    pub fn next(&mut self) -> Result<Option<(Vec<u8>, u32, u32)>> {
        match self.rx.recv() {
            Ok(buf) => Ok(Some((buf, self.dims.0, self.dims.1))),
            Err(_) => {
                take_err(&self.err)?;
                Ok(None)
            }
        }
    }

    pub fn recycle(&self, buf: Vec<u8>) {
        let _ = self.returner.send(buf);
    }

    pub fn join(self) -> Result<()> {
        let WebcamPipe {
            rx, handle, err, ..
        } = self;
        drop(rx);
        handle
            .join()
            .map_err(|_| anyhow!("webcam decode thread panicked"))?;
        take_err(&err)
    }
}

pub(crate) fn spawn_screen(
    mut dec: RawDecoder,
    pool: BufPool,
    tx: SyncSender<(Vec<u8>, usize)>,
    err: Arc<Mutex<Option<Error>>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut idx = 0usize;
        loop {
            let mut buf = pool.take();
            match dec.read_frame(&mut buf) {
                Ok(true) => {
                    if tx.send((buf, idx)).is_err() {
                        break;
                    }
                    idx += 1;
                }
                Ok(false) => break,
                Err(e) => {
                    *err.lock().unwrap() = Some(e);
                    break;
                }
            }
        }
    })
}

pub(crate) fn spawn_webcam(
    mut dec: RawDecoder,
    pool: BufPool,
    tx: SyncSender<Vec<u8>>,
    err: Arc<Mutex<Option<Error>>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        let mut buf = pool.take();
        match dec.read_frame(&mut buf) {
            Ok(true) => {
                if tx.send(buf).is_err() {
                    break;
                }
            }
            Ok(false) => break,
            Err(e) => {
                *err.lock().unwrap() = Some(e);
                break;
            }
        }
    })
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;

pub mod audio_mux;
pub mod audio_segments;
pub mod bg_pipe;
pub mod exporter;
pub mod ffio;
pub mod plan_walk;
pub mod run;
pub mod silence;
pub mod timeline;
