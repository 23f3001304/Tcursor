// Exporter: drives the composite stage of the 3-stage decode->composite->encode pipeline.
// Screen/webcam decode run on their own threads (pipeline.rs); this file owns the composite
// loop + encoder thread, joined by bounded channels with recycled buffer pools (pool.rs).
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::export::pipeline::audio_mux::mux;
use crate::export::pipeline::{ScreenPipe, WebcamPipe};
use crate::export::gpu::pool::BufPool;
use crate::export::render::{FrameRenderer, OUT_FPS};
use crate::export::types::Layout; // needed for Layout::default()
use crate::session::paths::ProjectPaths;

/// Render a recording into `paths.folder/final.mp4`. `on_progress` receives
/// 0..=100 as frames are encoded.
pub fn export(paths: &ProjectPaths, fps: u32, on_progress: impl Fn(u8)) -> Result<()> {
    let layout = Layout::default();
    let (mut r, meta) = FrameRenderer::new(paths, layout, fps)?;

    let screen_bytes = meta.screen_bytes;
    let size = meta.webcam_size;
    let wc_bytes = (size * size * 4) as usize;

    let tmp = paths.folder.join("tmp_export.mp4");
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let (out_w, out_h) = (meta.out_w, meta.out_h);
    let sink = FfmpegFrameSink::new_hq(tmp_str, out_w, out_h, OUT_FPS as f64)
        .context("create encode sink")?;

    let depth = 6; // covers the bounded channel (4) + in-flight buffers without allocating
    let out_bytes = (out_w * out_h * 4) as usize;
    let out_pool = BufPool::new(depth, out_bytes);
    let out_returner = out_pool.returner();

    let (tx, rx) = std::sync::mpsc::sync_channel::<Frame>(4);
    let encoder = std::thread::spawn(move || -> Result<()> {
        let mut sink = sink;
        for frame in rx {
            sink.push(&frame).context("encode push")?;
            let _ = out_returner.send(frame.bgra); // recycle the ~33MB output buffer
        }
        Box::new(sink).finish().context("finish encoder")?;
        Ok(())
    });

    let mut spipe = ScreenPipe::spawn(&paths.video(), screen_bytes, meta.tl.frames.clone(), depth)?;
    let mut wpipe = if paths.webcam().exists() {
        Some(WebcamPipe::spawn(&paths.webcam(), meta.video_start, size, wc_bytes, depth)?)
    } else { None };
    let empty = vec![0u8; screen_bytes]; // zero-frame fallback (matches old zeroed screen_buf)

    let total_out = (((meta.video_end - meta.video_start) * OUT_FPS) / 1000).max(1);
    let mut last_pct = u8::MAX;
    let export_start = std::time::Instant::now();
    let (mut t_dec, mut t_comp, mut t_send) = (0u128, 0u128, 0u128);
    for k in 0..=total_out {
        let t = meta.video_start + k * 1000 / OUT_FPS;
        let d0 = std::time::Instant::now();
        let screen = spipe.next_at(t)?.unwrap_or(&empty); // blocks on the screen decode channel
        let webcam = match &mut wpipe { Some(w) => w.next()?, None => None };
        t_dec += d0.elapsed().as_micros();
        let c0 = std::time::Instant::now();
        let pose = r.step_camera(t);
        let mut out = out_pool.take();
        let wc_ref = webcam.as_ref().map(|(b, w, h)| (b.as_slice(), *w, *h));
        r.composite_at(&pose, screen, wc_ref, &mut out);
        t_comp += c0.elapsed().as_micros();
        let s0 = std::time::Instant::now();
        let sent = tx.send(Frame { width: out_w, height: out_h, bgra: out, ts: Timestamp(t) }).is_ok();
        if let (Some(w), Some((buf, _, _))) = (wpipe.as_ref(), webcam) { w.recycle(buf); }
        t_send += s0.elapsed().as_micros();
        if !sent { break; }
        let pct = ((k * 100 / total_out).min(100)) as u8;
        if pct != last_pct { on_progress(pct); last_pct = pct; }
    }

    drop(tx);
    spipe.join()?;
    if let Some(w) = wpipe { w.join()?; }
    encoder.join().map_err(|_| anyhow!("encoder thread panicked"))??;
    let secs = export_start.elapsed().as_secs_f64().max(0.001);
    let _ = std::fs::write(std::env::temp_dir().join("tcursor-export-timing.txt"), format!(
        "frames={} total={:.2}s fps={:.1} decode={}ms composite={}ms encode_wait={}ms\n",
        total_out + 1, secs, (total_out + 1) as f64 / secs, t_dec / 1000, t_comp / 1000, t_send / 1000));
    let shift = |a: Option<u64>| a.map(|m| m as i64 - meta.video_start as i64).unwrap_or(0);
    let mic_shift = shift(meta.tl.mic_ms) + meta.audio_offset_ms as i64;
    mux(&tmp, paths, mic_shift, shift(meta.tl.system_ms))?;
    Ok(())
}

#[cfg(test)]
mod bench {
    // Headless export runner for M4 perf/correctness verification (byte-identical gate). Ignored by
    // default; run against a real recording folder with:
    //   TCURSOR_REC=<folder> cargo test --lib export::pipeline::exporter::bench::export_bench -- --ignored --nocapture
    // then framemd5 the folder's final.mp4 to compare before/after a task.
    #[test]
    #[ignore]
    fn export_bench() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let t = std::time::Instant::now();
        super::export(&paths, 60, |_| {}).expect("export failed");
        eprintln!("export_bench: exported {folder} in {:?}", t.elapsed());
    }
}
