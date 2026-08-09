//! Decode-thread bodies for `pipeline.rs`, split out to keep each file <=200 lines.
//! Each spawns an ffmpeg `RawDecoder` loop that reads frames into pooled buffers and
//! streams them over a bounded channel; a stored `Error` distinguishes a decode
//! failure from a clean EOF (the loop just drops its sender on EOF).
use anyhow::Error;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use crate::export::pipeline::ffio::RawDecoder;
use crate::export::gpu::pool::BufPool;

/// Screen decode loop: stream `(buf, idx)` for every decoded captured frame, taking
/// buffers from `pool`. Stops (dropping `tx`) on EOF, on a closed channel, or after
/// storing a decode error in `err`.
pub(crate) fn spawn_screen(mut dec: RawDecoder, pool: BufPool,
    tx: SyncSender<(Vec<u8>, usize)>, err: Arc<Mutex<Option<Error>>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut idx = 0usize;
        loop {
            let mut buf = pool.take();
            match dec.read_frame(&mut buf) {
                Ok(true) => { if tx.send((buf, idx)).is_err() { break; } idx += 1; }
                Ok(false) => break,
                Err(e) => { *err.lock().unwrap() = Some(e); break; }
            }
        }
    })
}

/// Webcam decode loop: stream one buffer per output frame (1:1). Stops (dropping `tx`) on
/// EOF, on a closed channel, or after storing a decode error. The frame's dimensions are
/// fixed for the whole export (the decoder cover-crops to them), so `WebcamPipe` holds them
/// rather than repeating them on every message.
pub(crate) fn spawn_webcam(mut dec: RawDecoder, pool: BufPool,
    tx: SyncSender<Vec<u8>>, err: Arc<Mutex<Option<Error>>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let mut buf = pool.take();
            match dec.read_frame(&mut buf) {
                Ok(true) => { if tx.send(buf).is_err() { break; } }
                Ok(false) => break,
                Err(e) => { *err.lock().unwrap() = Some(e); break; }
            }
        }
    })
}
