// Exporter: drives the composite stage of the 3-stage decode->composite->encode pipeline.
// Screen/webcam decode run on their own threads (pipeline.rs); this file owns the composite
// loop + encoder thread, joined by bounded channels with recycled buffer pools (pool.rs).
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::export::pipeline::audio_mux::mux;
use crate::export::pipeline::{trim_frame_bounds, ScreenPipe, WebcamPipe};
use crate::export::gpu::pool::BufPool;
use crate::export::render::FrameRenderer;
use crate::export::settings::ExportSettings;
use crate::export::types::Layout; // needed for Layout::default()
use crate::session::paths::ProjectPaths;

/// Render a recording into `paths.folder/final.<ext>` (`<ext>` from `settings.format`).
/// `on_progress` receives 0..=100 as frames are encoded. `settings` resolves the output
/// resolution (combined with the doc's own `Aspect`), frame rate, quality, and container -
/// `ExportSettings::default()` reproduces today's export exactly (Source/60fps/CRF 24/MP4).
pub fn export(paths: &ProjectPaths, settings: ExportSettings, on_progress: impl Fn(u8)) -> Result<()> {
    // The SAME display-refresh-derived value the old unconditional formula used, both as
    // `FrameRenderer::new`'s capture-rate fallback (`build_timeline`, unchanged meaning) and as
    // `Fps::Source`'s own fallback (`resolve_hz`) - so `Source` reproduces the old formula
    // exactly without a second display query.
    let capture_fps = crate::win::sys::display::primary_refresh_hz().min(60);
    let out_fps = settings.fps.resolve_hz(capture_fps);
    let layout = Layout::default();
    let (mut r, meta) = FrameRenderer::new(paths, layout, capture_fps, settings.resolution, None)?;

    let screen_bytes = meta.screen_bytes;
    let size = meta.webcam_size;
    let wc_bytes = (size * size * 4) as usize;

    let (out_w, out_h) = (meta.out_w, meta.out_h);
    let tmp = paths.folder.join(format!("tmp_export.{}", settings.format.extension()));
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let sink = FfmpegFrameSink::new_medium(tmp_str, out_w, out_h, out_fps as f64, settings.format, settings.quality_crf)
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

    let mut spipe = ScreenPipe::spawn(&paths.video(), screen_bytes, None, depth, out_fps)?;
    let mut wpipe = if paths.webcam().exists() {
        Some(WebcamPipe::spawn(&paths.webcam(), meta.video_start, size, wc_bytes, depth, out_fps)?)
    } else { None };
    // Zero-frame fallback as a black nv12 frame (Y=16, U=V=128); a zeroed buffer would decode to a
    // green tint through the color convert. Only used if the screen decode yields nothing.
    let mut empty = vec![16u8; screen_bytes];
    for b in &mut empty[(meta.sw as usize * meta.sh as usize)..] { *b = 128; }

    // Trim gates which output frames are actually composited/encoded: `[k_in, k_last]` (inclusive
    // frame indices at `out_fps`) is the resolved trim range (`out_ms == 0` = whole clip, see
    // `Trim::resolve`); an untrimmed clip yields the exact same `k_last` the old unconditional
    // loop bound computed, so nothing changes when there is no trim. The loop still runs to
    // `k_full_last` (the untrimmed clip's own last index) so the screen/webcam decode threads and
    // the camera sim advance in the same lockstep they always have - only the expensive
    // composite+encode step is skipped outside the trim range.
    let full_dur_ms = ((meta.video_end - meta.video_start) as u32).max(1);
    let (trim_in_ms, trim_out_ms) = meta.trim.resolve(full_dur_ms);
    let (k_in, k_last) = trim_frame_bounds(trim_in_ms, trim_out_ms, out_fps);
    let k_full_last = ((full_dur_ms as u64 * out_fps) / 1000).max(k_last);
    let total_out = k_last - k_in + 1; // trimmed frame count - drives progress % and the log lines below
    eprintln!("[EXPORT] Starting export for {:?} (target {}x{} @ {}FPS, total_frames={}, trim={}..{}ms of {}ms)",
        paths.folder, out_w, out_h, out_fps, total_out, trim_in_ms, trim_out_ms, full_dur_ms);
    let mut last_pct = u8::MAX;
    let export_start = std::time::Instant::now();
    let (mut t_dec, mut t_comp, mut t_send) = (0u128, 0u128, 0u128);
    // Hold the most-recent webcam frame: if the webcam stream is shorter than the screen (dual-
    // stream start/stop timing, or a lower webcam frame count), `next()` returns None for the tail
    // and the PiP would vanish early - instead we freeze it on the last decoded frame. Buffers are
    // pooled (depth 6), so keeping one held out of the pool costs nothing.
    let mut last_webcam: Option<(Vec<u8>, u32, u32)> = None;
    for k in 0..=k_full_last {
        let t = meta.video_start + k * 1000 / out_fps;
        let d0 = std::time::Instant::now();
        let screen = spipe.next()?.unwrap_or(&empty); // decoded at -r out_fps, so output frame k IS decoded frame k at any export rate
        if let Some(w) = &mut wpipe {
            if let Some(next) = w.next()? {
                if let Some((old, _, _)) = last_webcam.take() { w.recycle(old); }
                last_webcam = Some(next);
            } // else: webcam EOF - keep last_webcam and hold it for the rest of the export
        }
        t_dec += d0.elapsed().as_micros();
        let pose = r.step_camera(t); // always advance camera/cursor state, even outside the trim range
        if k < k_in { continue; } // still decoded/stepped above for continuity, just not composited
        if k > k_last { break; } // past trim-out: stop entirely (decode threads join below)
        let c0 = std::time::Instant::now();
        let mut out = out_pool.take();
        let wc_ref = last_webcam.as_ref().map(|(b, w, h)| (b.as_slice(), *w, *h));
        r.composite_at(&pose, screen, wc_ref, &mut out);
        t_comp += c0.elapsed().as_micros();
        let s0 = std::time::Instant::now();
        let sent = tx.send(Frame { width: out_w, height: out_h, bgra: out, ts: Timestamp(t) }).is_ok();
        t_send += s0.elapsed().as_micros();
        if !sent { break; }
        // `done` counts completed frames (1-based), so it - unlike the raw index `k - k_in` -
        // reaches exactly `total_out` (100%) on the last frame actually sent.
        let done = k - k_in + 1;
        let pct = ((done * 100 / total_out).min(100)) as u8;
        if pct != last_pct {
            on_progress(pct);
            if pct % 5 == 0 {
                let elapsed = export_start.elapsed().as_secs_f64();
                let fps_curr = done as f64 / elapsed.max(0.001);
                eprintln!("[EXPORT] Progress: {:3}% | frame {:5}/{} | speed: {:.1} FPS | elapsed: {:.1}s", pct, done, total_out, fps_curr, elapsed);
            }
            last_pct = pct;
        }
    }
    if let (Some(w), Some((buf, _, _))) = (wpipe.as_ref(), last_webcam) { w.recycle(buf); }

    drop(tx);
    spipe.join()?;
    if let Some(w) = wpipe { w.join()?; }
    encoder.join().map_err(|_| anyhow!("encoder thread panicked"))??;
    let secs = export_start.elapsed().as_secs_f64().max(0.001);
    eprintln!("[EXPORT] Finished render: {} frames in {:.2}s ({:.1} FPS)", total_out, secs, total_out as f64 / secs);
    let _ = std::fs::write(std::env::temp_dir().join("tcursor-export-timing.txt"), format!(
        "frames={} total={:.2}s fps={:.1} decode={}ms composite={}ms encode_wait={}ms\n",
        total_out, secs, total_out as f64 / secs, t_dec / 1000, t_comp / 1000, t_send / 1000));
    // Audio aligns to the video's own frame 0; once trimmed, that frame sits at `trim_in_ms`
    // into the original capture, so both tracks shift earlier by the same amount to stay in sync.
    let shift = |a: Option<u64>| a.map(|m| m as i64 - meta.video_start as i64).unwrap_or(0) - trim_in_ms as i64;
    let mic_shift = shift(meta.tl.mic_ms) + meta.audio_offset_ms as i64;
    // Cap the muxed audio to the trimmed video's own duration, so a trim-out doesn't leave a
    // longer source audio file playing past the video's frozen last frame.
    let out_dur_ms = (total_out * 1000) / out_fps;
    mux(&tmp, paths, settings.format, mic_shift, shift(meta.tl.system_ms), meta.mic_volume, meta.sys_volume, out_dur_ms)?;
    Ok(())
}

#[cfg(test)]
mod bench {
    #[test]
    #[ignore]
    fn export_bench() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let paths = crate::session::paths::ProjectPaths { folder: std::path::PathBuf::from(&folder) };
        let t = std::time::Instant::now();
        super::export(&paths, crate::export::settings::ExportSettings::default(), |_| {}).expect("export failed");
        eprintln!("export_bench: exported {folder} in {:?}", t.elapsed());
    }
}
