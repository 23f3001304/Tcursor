// Exporter: drives the composite stage of the 3-stage decode->composite->encode pipeline.
// Screen/webcam decode run on their own threads (pipeline.rs); this file owns the composite
// loop + encoder thread, joined by bounded channels with recycled buffer pools (pool.rs).
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::export::pipeline::audio_mux::mux;
use crate::export::pipeline::bg_pipe::BgPipe;
use crate::export::pipeline::{audio_shift_ms, webcam_warning, ScreenPipe, WebcamPipe};
use crate::export::scene::background::video_source;
use crate::export::gpu::pool::BufPool;
use crate::export::render::FrameRenderer;
use crate::export::settings::ExportSettings;
use crate::export::types::Layout; // needed for Layout::default()
use crate::session::paths::ProjectPaths;

/// Render a recording into `paths.folder/final.<ext>` (`<ext>` from `settings.format`).
/// `on_progress` receives 0..=100 as frames are encoded. `settings` resolves the output
/// resolution (combined with the doc's own `Aspect`), frame rate, quality, and container -
/// `ExportSettings::default()` reproduces today's export exactly (Source/60fps/CRF 24/MP4).
///
/// Returns the export's non-fatal WARNINGS (`run_export` emits each as an `export-warning`):
/// things that produced a real file but not the one the user expected, and which used to be
/// swallowed entirely - today that is a webcam that failed to decode or decoded zero frames.
/// A failure that makes the file worthless (the screen decode dying, or producing nothing at
/// all) is an `Err` instead.
pub fn export(paths: &ProjectPaths, settings: ExportSettings, on_progress: impl Fn(u8)) -> Result<Vec<String>> {
    // The SAME display-refresh-derived value the old unconditional formula used, both as
    // `FrameRenderer::new`'s capture-rate fallback (`build_timeline`, unchanged meaning) and as
    // `Fps::Source`'s own fallback (`resolve_hz`) - so `Source` reproduces the old formula
    // exactly without a second display query.
    let capture_fps = crate::win::sys::display::primary_refresh_hz().min(60);
    let out_fps = settings.fps.resolve_hz(capture_fps);
    let layout = Layout::default();
    let (mut r, meta) = FrameRenderer::new(paths, layout, capture_fps, settings.resolution, None)?;

    let screen_bytes = meta.screen_bytes;
    let wc_dims = (meta.webcam_w, meta.webcam_h); // source-aspect decode box; panels crop it at composite time
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;

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
            let _written = sink.push(&frame).context("encode push")?; // export dims never change; skip never happens here
            let _ = out_returner.send(frame.bgra); // recycle the ~33MB output buffer
        }
        Box::new(sink).finish().context("finish encoder")?;
        Ok(())
    });

    let spipe = ScreenPipe::spawn(&paths.video(), screen_bytes, meta.screen_crop, None, depth, out_fps)?;
    let wpipe = if paths.webcam().exists() {
        Some(WebcamPipe::spawn(&paths.webcam(), meta.video_start, wc_dims, wc_bytes, depth, out_fps)?)
    } else { None };
    let bgpipe = video_source(r.background(), &paths.folder).and_then(|src|
        BgPipe::open(&src, (out_w, out_h), r.background().dim_clamped(), depth, out_fps)
            .map_err(|e| eprintln!("[EXPORT] background video: {e} - using its still first frame")).ok());
    let mut pipes = frame_loop::Pipes { spipe, wpipe, bgpipe, last_webcam: None, wc_fail: None, wc_frames: 0, held: None };

    // The frame plan names the recording frame every output frame shows (`TimeMap::frame_plan`): a
    // trim-only doc gives exactly the old `k_in..=k_last`; cuts and speed spans skip or repeat frames.
    let map = r.time_map().clone();
    let plan = map.frame_plan(out_fps);
    if plan.is_empty() { return Err(anyhow!("everything is cut - there is nothing to export")); }
    let total_out = plan.len() as u64;
    eprintln!("[EXPORT] Starting export for {:?} (target {}x{} @ {}FPS, total_frames={}, {} kept segment(s) of {}ms)",
        paths.folder, out_w, out_h, out_fps, total_out, map.segments().len(), meta.video_end - meta.video_start);
    let export_start = std::time::Instant::now();
    let clock = frame_loop::Clock { video_start: meta.video_start, out_fps };
    let (sent, timing) = frame_loop::run(&mut r, &mut pipes, &plan, &paths.video(), &clock, (out_w, out_h), &out_pool, &tx, &on_progress)?;
    let frame_loop::Pipes { spipe, wpipe, bgpipe, last_webcam, mut wc_fail, wc_frames, held: _ } = pipes;
    let had_webcam = wpipe.is_some();
    if let (Some(w), Some((buf, _, _))) = (wpipe.as_ref(), last_webcam) { w.recycle(buf); }

    drop(tx);
    spipe.join()?;
    if let Some(e) = bgpipe.and_then(|b| b.join().err()) { eprintln!("[EXPORT] background video: {e}"); }
    // A webcam error that only surfaces at JOIN is the same non-fatal case as one seen mid-loop.
    if let Some(w) = wpipe {
        if let Err(e) = w.join() { if wc_fail.is_none() { wc_fail = Some(e.to_string()); } }
    }
    encoder.join().map_err(|_| anyhow!("encoder thread panicked"))??;
    let warning = webcam_warning(had_webcam, wc_fail.as_deref(), wc_frames);
    if let Some(w) = &warning { eprintln!("[EXPORT] WARNING: {w} (webcam: {:?})", paths.webcam()); }
    exporter_report::log_timing(sent, export_start.elapsed().as_secs_f64().max(0.001), timing.dec, timing.comp, timing.send);
    // Audio aligns to output frame 0, which shows recording frame `plan[0]`: both tracks shift
    // earlier by that frame-floored amount (see `audio_shift_ms`), exactly as the trim did.
    let trim_in_q = plan[0] * 1000 / out_fps;
    let shift = |a: Option<u64>| audio_shift_ms(a, meta.video_start, trim_in_q);
    let mic_shift = shift(meta.tl.mic_ms) + meta.audio_offset_ms as i64;
    let out_dur_ms = (sent * 1000) / out_fps;
    let segs = crate::export::pipeline::audio_segments::audio_segs(&map, out_fps, trim_in_q);
    mux(&tmp, paths, settings.format, mic_shift, shift(meta.tl.system_ms), meta.mic_volume, meta.sys_volume, out_dur_ms, &segs)?;
    Ok(warning.into_iter().collect())
}

// Progress + timing reporting (no effect on a pixel), split out for the line budget.
#[path = "exporter_report.rs"]
mod exporter_report;
#[path = "frame_loop.rs"]
mod frame_loop;

#[cfg(test)]
#[path = "exporter_bench.rs"]
mod bench;
