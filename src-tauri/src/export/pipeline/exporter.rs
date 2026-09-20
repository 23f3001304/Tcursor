use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::audio_mux::mux;
use crate::export::pipeline::bg_pipe::BgPipe;
use crate::export::pipeline::{audio_shift_ms, webcam_warning, ScreenPipe, WebcamPipe};
use crate::export::render::FrameRenderer;
use crate::export::scene::background::video_source;
use crate::export::settings::ExportSettings;
use crate::export::types::Layout;
use crate::ports::system::SystemPort;
use crate::session::paths::ProjectPaths;

pub fn export(
    paths: &ProjectPaths,
    settings: ExportSettings,
    system: &dyn SystemPort,
    on_progress: impl Fn(u8),
) -> Result<Vec<String>> {
    let capture_fps = system.primary_refresh_hz().min(60);
    let out_fps = settings.fps.resolve_hz(capture_fps);
    let layout = Layout::default();
    let (mut r, meta) = FrameRenderer::new(
        paths,
        layout,
        capture_fps,
        settings.resolution,
        None,
        system,
    )?;

    let screen_bytes = meta.screen_bytes;
    let wc_dims = (meta.webcam_w, meta.webcam_h);
    let wc_bytes = (wc_dims.0 * wc_dims.1 * 4) as usize;

    let (out_w, out_h) = (meta.out_w, meta.out_h);
    let tmp = paths
        .folder
        .join(format!("tmp_export.{}", settings.format.extension()));
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let sink = FfmpegFrameSink::new_medium(
        tmp_str,
        out_w,
        out_h,
        out_fps as f64,
        settings.format,
        settings.quality_crf,
    )
    .context("create encode sink")?;

    let depth = 6;
    let out_bytes = (out_w * out_h * 4) as usize;
    let out_pool = BufPool::new(depth, out_bytes);
    let out_returner = out_pool.returner();

    let (tx, rx) = std::sync::mpsc::sync_channel::<Frame>(4);
    let encoder = std::thread::spawn(move || -> Result<()> {
        let mut sink = sink;
        for frame in rx {
            let _written = sink.push(&frame).context("encode push")?;
            let _ = out_returner.send(frame.bgra);
        }
        Box::new(sink).finish().context("finish encoder")?;
        Ok(())
    });

    let spipe = ScreenPipe::spawn(
        &paths.video(),
        screen_bytes,
        meta.screen_crop,
        None,
        depth,
        out_fps,
        None,
    )?;
    let wpipe = if paths.webcam().exists() {
        Some(WebcamPipe::spawn(
            &paths.webcam(),
            meta.video_start,
            wc_dims,
            wc_bytes,
            depth,
            out_fps,
        )?)
    } else {
        None
    };
    let bgpipe = video_source(r.background(), &paths.folder).and_then(|src| {
        BgPipe::open(
            &src,
            (out_w, out_h),
            r.background().dim_clamped(),
            depth,
            out_fps,
        )
        .map_err(|e| eprintln!("[EXPORT] background video: {e} - using its still first frame"))
        .ok()
    });
    let mut pipes = frame_loop::Pipes {
        spipe,
        wpipe,
        bgpipe,
        last_webcam: None,
        wc_fail: None,
        wc_frames: 0,
        held: None,
        clip_held: None,
        mix_scratch: Vec::new(),
    };

    let map = r.time_map().clone();
    let plan = map.frame_plan(out_fps);
    if plan.is_empty() {
        return Err(anyhow!("everything is cut - there is nothing to export"));
    }
    let spans = map.clip_spans(out_fps);
    let total_out = plan.len() as u64;
    eprintln!("[EXPORT] Starting export for {:?} (target {}x{} @ {}FPS, total_frames={}, {} kept segment(s) in {} clip(s) of {}ms)",
        paths.folder, out_w, out_h, out_fps, total_out, map.segments().len(), spans.len(), meta.video_end - meta.video_start);
    let export_start = std::time::Instant::now();
    let clock = frame_loop::Clock {
        video_start: meta.video_start,
        out_fps,
    };
    let dec = frame_loop::ClipDecode {
        video: paths.video(),
        webcam: paths.webcam().exists().then(|| paths.webcam()),
        screen_bytes,
        screen_crop: meta.screen_crop,
        wc_dims,
        wc_bytes,
        depth,
        sw: meta.sw,
        sh: meta.sh,
    };
    let (sent, timing) = frame_loop::run(
        &mut r,
        &mut pipes,
        &plan,
        &spans,
        &dec,
        &paths.video(),
        &clock,
        (out_w, out_h),
        &out_pool,
        &tx,
        &on_progress,
    )?;
    let frame_loop::Pipes {
        spipe,
        wpipe,
        bgpipe,
        last_webcam,
        mut wc_fail,
        wc_frames,
        held: _,
        clip_held: _,
        mix_scratch: _,
    } = pipes;
    let had_webcam = wpipe.is_some();
    if let (Some(w), Some((buf, _, _))) = (wpipe.as_ref(), last_webcam) {
        w.recycle(buf);
    }

    drop(tx);
    spipe.join()?;
    if let Some(e) = bgpipe.and_then(|b| b.join().err()) {
        eprintln!("[EXPORT] background video: {e}");
    }
    if let Some(w) = wpipe {
        if let Err(e) = w.join() {
            if wc_fail.is_none() {
                wc_fail = Some(e.to_string());
            }
        }
    }
    encoder
        .join()
        .map_err(|_| anyhow!("encoder thread panicked"))??;
    let warning = webcam_warning(had_webcam, wc_fail.as_deref(), wc_frames);
    if let Some(w) = &warning {
        eprintln!("[EXPORT] WARNING: {w} (webcam: {:?})", paths.webcam());
    }
    exporter_report::log_timing(
        sent,
        export_start.elapsed().as_secs_f64().max(0.001),
        timing.dec,
        timing.comp,
        timing.send,
    );
    let trim_in_q = crate::export::pipeline::audio_segments::audio_origin_q(&plan, out_fps);
    let shift = |a: Option<u64>| audio_shift_ms(a, meta.video_start, trim_in_q);
    let mic_shift = shift(meta.tl.mic_ms) + meta.audio_offset_ms as i64;
    let out_dur_ms = (sent * 1000) / out_fps;
    let segs = crate::export::pipeline::audio_segments::audio_segs(&map, out_fps, trim_in_q);
    mux(
        &tmp,
        paths,
        settings.format,
        mic_shift,
        shift(meta.tl.system_ms),
        meta.mic_volume,
        meta.sys_volume,
        out_dur_ms,
        &segs,
    )?;
    Ok(warning.into_iter().collect())
}

#[path = "exporter_report.rs"]
mod exporter_report;
#[path = "frame_loop.rs"]
mod frame_loop;

#[cfg(test)]
mod bench {
    #[test]
    #[ignore]
    fn preview_frame_bench() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let ms: u32 = std::env::var("TCURSOR_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1500);
        let paths = crate::session::paths::ProjectPaths {
            folder: std::path::PathBuf::from(&folder),
        };
        let platform = crate::platform::current();
        let png = crate::export::preview::render_preview(&paths, ms, platform.system.as_ref())
            .expect("render failed");
        let out = std::env::temp_dir().join("tcursor-preview-frame.png");
        std::fs::write(&out, png).expect("write png");
        eprintln!(
            "preview_frame_bench: frame at {ms}ms of {folder} -> {}",
            out.display()
        );
    }

    #[test]
    #[ignore]
    fn export_bench() {
        let folder = std::env::var("TCURSOR_REC").expect("set TCURSOR_REC to a recording folder");
        let paths = crate::session::paths::ProjectPaths {
            folder: std::path::PathBuf::from(&folder),
        };
        let t = std::time::Instant::now();
        let platform = crate::platform::current();
        super::export(
            &paths,
            crate::export::settings::ExportSettings::default(),
            platform.system.as_ref(),
            |_| {},
        )
        .expect("export failed");
        eprintln!("export_bench: exported {folder} in {:?}", t.elapsed());
    }
}
