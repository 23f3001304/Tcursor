// Exporter: drives the frame loop with FrameRenderer, encodes to tmp mp4, muxes audio.
// All per-frame render logic lives in render.rs; this file owns only the decoder/encoder.
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::export::audio_mux::mux;
use crate::export::ffio::RawDecoder;
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
    let mut screen_dec = RawDecoder::spawn(&paths.video(), 0.0, false, None, None, screen_bytes)?;
    let mut webcam_dec = if paths.webcam().exists() {
        Some(RawDecoder::spawn(&paths.webcam(), OUT_FPS as f64, false,
            Some(meta.video_start), Some(size), wc_bytes)?)
    } else { None };

    let tmp = paths.folder.join("tmp_export.mp4");
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let (out_w, out_h) = (meta.out_w, meta.out_h);
    let sink = FfmpegFrameSink::new_hq(tmp_str, out_w, out_h, OUT_FPS as f64)
        .context("create encode sink")?;
    let (tx, rx) = std::sync::mpsc::sync_channel::<Frame>(4);
    let encoder = std::thread::spawn(move || -> Result<()> {
        let mut sink = sink;
        for frame in rx { sink.push(&frame).context("encode push")?; }
        Box::new(sink).finish().context("finish encoder")?;
        Ok(())
    });

    let total_out = (((meta.video_end - meta.video_start) * OUT_FPS) / 1000).max(1);
    let mut screen_buf = vec![0u8; screen_bytes];
    let mut wc_buf = vec![0u8; wc_bytes];
    let mut last_pct = u8::MAX;
    let mut cap_idx = 0usize;
    let mut have = screen_dec.read_frame(&mut screen_buf)?;

    let export_start = std::time::Instant::now();
    let (mut t_dec, mut t_comp, mut t_send) = (0u128, 0u128, 0u128);
    for k in 0..=total_out {
        let t = meta.video_start + k * 1000 / OUT_FPS;
        let d0 = std::time::Instant::now();
        while have && cap_idx + 1 < meta.tl.frames.len() && meta.tl.frames[cap_idx + 1] <= t {
            have = screen_dec.read_frame(&mut screen_buf)?;
            if have { cap_idx += 1; }
        }
        let webcam = read_webcam(&mut webcam_dec, &mut wc_buf, size)?;
        t_dec += d0.elapsed().as_micros();
        let c0 = std::time::Instant::now();
        let pose = r.step_camera(t);
        let out = r.composite_at(&pose, &screen_buf, webcam);
        t_comp += c0.elapsed().as_micros();
        let s0 = std::time::Instant::now();
        if tx.send(Frame { width: out_w, height: out_h, bgra: out, ts: Timestamp(t) }).is_err() {
            break;
        }
        t_send += s0.elapsed().as_micros();
        let pct = ((k * 100 / total_out).min(100)) as u8;
        if pct != last_pct { on_progress(pct); last_pct = pct; }
    }

    drop(tx);
    drop(screen_dec);
    drop(webcam_dec);
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

/// Read one webcam frame; on EOF drop the decoder so later frames have none.
fn read_webcam<'a>(
    dec: &mut Option<RawDecoder>,
    buf: &'a mut [u8],
    size: u32,
) -> Result<Option<(&'a [u8], u32, u32)>> {
    let still = match dec { Some(d) => d.read_frame(buf)?, None => false };
    if !still { *dec = None; return Ok(None); }
    Ok(Some((&*buf, size, size)))
}
