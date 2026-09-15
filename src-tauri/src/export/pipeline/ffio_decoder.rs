use crate::process::proc::ffcmd;
use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

const TAIL_CAP: usize = 2000;

pub struct RawDecoder {
    child: Child,
    stdout: ChildStdout,
    frame_bytes: usize,
    stderr: Arc<Mutex<String>>,
    drain: Option<JoinHandle<()>>,
    frames: u64,
}

fn decode_args(
    video: &Path,
    rate: f64,
    input_rate: bool,
    seek_ms: Option<u64>,
    crop: Option<(u32, u32)>,
    cover_scale: Option<(u32, u32)>,
    target_dims: Option<(u32, u32)>,
    pix_fmt: &str,
) -> Vec<String> {
    let r = format!("{rate:.4}");
    let mut args: Vec<String> = ["-v", "error", "-hwaccel", "auto"].map(String::from).into();
    if let Some(ms) = seek_ms {
        args.push("-ss".into());
        args.push(format!("{:.4}", ms as f64 / 1000.0));
    }
    if rate > 0.0 && input_rate {
        args.push("-r".into());
        args.push(r.clone());
    }
    args.push("-i".into());
    args.push(video.to_string_lossy().into_owned());
    if rate > 0.0 && !input_rate {
        args.push("-r".into());
        args.push(r);
    }
    args.push("-sws_flags".into());
    args.push("fast_bilinear".into());
    let mut vf: Vec<String> = Vec::new();
    if let Some((w, h)) = crop {
        vf.push(format!("crop={w}:{h}:0:0"));
    }
    if let Some((w, h)) = target_dims {
        vf.push(format!("scale={w}:{h}:flags=fast_bilinear"));
    } else if let Some((w, h)) = cover_scale {
        vf.push(format!(
            "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
        ));
    }
    if !vf.is_empty() {
        args.push("-vf".into());
        args.push(vf.join(","));
    }
    args.push("-f".into());
    args.push("rawvideo".into());
    args.push("-pix_fmt".into());
    args.push(pix_fmt.to_string());
    args.push("-".into());
    args
}

// Drain ffmpeg's stderr into a rolling tail on its own thread. A THREAD, not a read after
// EOF: ffmpeg blocks once the ~64 KB stderr pipe fills and then stops writing stdout too, so
// a reader waiting on the next frame would deadlock against the very message it is waiting for.
fn drain_stderr(pipe: ChildStderr, sink: Arc<Mutex<String>>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            let mut t = sink.lock().unwrap_or_else(|e| e.into_inner());
            if t.len() + line.len() > TAIL_CAP {
                t.clear();
            }
            t.push_str(&line);
            t.push('\n');
        }
    })
}

fn classify_end(success: bool, status: &str, frames: u64, tail: &str) -> Result<bool> {
    if success {
        return Ok(false);
    }
    let why = if tail.is_empty() {
        "no stderr output"
    } else {
        tail
    };
    Err(anyhow!(
        "ffmpeg decode failed ({status}) after {frames} frame(s): {why}"
    ))
}

impl RawDecoder {
    pub fn spawn(
        video: &Path,
        rate: f64,
        input_rate: bool,
        seek_ms: Option<u64>,
        crop: Option<(u32, u32)>,
        cover_scale: Option<(u32, u32)>,
        target_dims: Option<(u32, u32)>,
        pix_fmt: &str,
        frame_bytes: usize,
    ) -> Result<Self> {
        Self::spawn_args(
            decode_args(
                video,
                rate,
                input_rate,
                seek_ms,
                crop,
                cover_scale,
                target_dims,
                pix_fmt,
            ),
            frame_bytes,
        )
    }

    pub fn spawn_args(args: Vec<String>, frame_bytes: usize) -> Result<Self> {
        let mut child = ffcmd("ffmpeg")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawn ffmpeg decoder")?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("decoder stdout missing"))?;
        let stderr = Arc::new(Mutex::new(String::new()));
        let drain = child.stderr.take().map(|p| drain_stderr(p, stderr.clone()));
        Ok(Self {
            child,
            stdout,
            frame_bytes,
            stderr,
            drain,
            frames: 0,
        })
    }

    pub fn read_frame(&mut self, buf: &mut [u8]) -> Result<bool> {
        debug_assert_eq!(buf.len(), self.frame_bytes);
        match self.stdout.read_exact(buf) {
            Ok(()) => {
                self.frames += 1;
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => self.end_of_stream(),
            Err(e) => Err(anyhow!("decoder read failed: {e}")),
        }
    }

    fn end_of_stream(&mut self) -> Result<bool> {
        let st = self.child.wait().context("wait ffmpeg decoder")?;
        if let Some(h) = self.drain.take() {
            let _ = h.join();
        }
        let tail = self
            .stderr
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .trim()
            .to_string();
        classify_end(st.success(), &st.to_string(), self.frames, &tail)
    }
}

impl Drop for RawDecoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "ffio_decoder_tests.rs"]
mod tests;
