// Exporter: decode the screen + webcam, composite each frame with auto-zoom,
// encode to a temp mp4, then mux audio into final.mp4. Frames are placed by
// their real capture timestamp (from the recording's sync log), so any capture
// rate — fixed, mislabeled, or variable — reconstructs to the right timeline.
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::events::model::EventLog;
use crate::export::audio_mux::mux;
use crate::export::background;
use crate::export::camera::CameraSim;
use crate::export::compositor::{Compositor, CpuCompositor};
use crate::export::coordmap::to_panel;
use crate::export::cursor::Cursor;
use crate::export::ffio::{decode_image, probe_dims, RawDecoder};
use crate::export::timeline::build_timeline;
use crate::export::types::{Background, Camera, Layout, OverlayLayout};
use crate::export::{autozoom, gpu, gpu_compositor::GpuCompositor};
use crate::session::paths::ProjectPaths;

/// Bundled default background (a macOS-style gradient wallpaper).
const BG_MESH: &[u8] = include_bytes!("../../assets/backgrounds/bg.jpg");

/// Constant output frame rate; the variable-rate capture is resampled to this.
const OUT_FPS: u64 = 60;

/// Render a recording into `paths.folder/final.mp4`. `on_progress` receives
/// 0..=100 as frames are encoded.
pub fn export(paths: &ProjectPaths, fps: u32, on_progress: impl Fn(u8)) -> Result<()> {
    let log = EventLog::load(&paths.events()).context("load events.json")?;
    let settings: crate::settings::model::Settings = std::fs::read(paths.settings())
        .ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let cfg = settings.zoom.to_zoom_config();
    let video = paths.video();
    let (sw, sh) = probe_dims(&video)?;
    let layout = Layout::default();
    const TRANSITION_MS: u32 = 350;
    let actions = crate::actions::model::ActionLog::load(&paths.actions())
        .map(|a| a.actions).unwrap_or_default();
    let ov = OverlayLayout::default();
    let track = crate::export::layout::LayoutTrack::new(&actions, &layout, &ov, sw, sh, TRANSITION_MS);

    // Click (auto) regions, then manual hold-to-zoom regions, merged into one list
    // and re-anchored into each region's active screen panel. Manual regions come
    // LAST so an active manual hold takes precedence (CameraSim picks the most-
    // recent active region). Manual holds apply even when click-zoom is disabled.
    let mut raw = if settings.zoom.enabled {
        autozoom::generate(&log.events, &log.screen, &cfg)
    } else {
        Vec::new()
    };
    raw.extend(crate::export::manual::from_actions(&actions, &log.events, &log.screen, &cfg));
    let regions = crate::export::layout::anchor_regions(raw, &track, sw, sh);

    // The real capture timeline (ms per frame) + audio/event start offsets.
    let tl = build_timeline(paths, &log, fps);
    let video_start = tl.frames[0];
    let video_end = (*tl.frames.last().unwrap_or(&video_start)).max(video_start + 1);
    let total_out = (((video_end - video_start) * OUT_FPS) / 1000).max(1);

    let bg = decode_image(BG_MESH, layout.out_w, layout.out_h)
        .unwrap_or_else(|_| background::render(&Background::default(), layout.out_w, layout.out_h));
    let compositor = select_compositor(&layout);
    let mut sim = CameraSim::new(layout.out_w, layout.out_h);
    let mut cursor = Cursor::new(&log.events, &log.screen);

    // Screen decodes natively; we sample it by timestamp. Webcam is resampled to
    // the output rate with its lead trimmed to the video start (so it lip-syncs).
    let screen_bytes = (sw * sh * 4) as usize;
    let mut screen_dec = RawDecoder::spawn(&video, 0.0, false, None, None, screen_bytes)?;
    // Decode the webcam large enough for the biggest camera panel (the centered
    // square in CameraFocus/CameraOnly), capped to bound memory; panels resize down.
    let pad2 = 2 * layout.pad_px;
    let size = layout.out_w.saturating_sub(pad2).min(layout.out_h.saturating_sub(pad2)).min(1440).max(ov.size_px);
    let wc_bytes = (size * size * 4) as usize;
    let mut webcam_dec = if paths.webcam().exists() {
        Some(RawDecoder::spawn(&paths.webcam(), OUT_FPS as f64, false, Some(video_start), Some(size), wc_bytes)?)
    } else {
        None
    };

    let tmp = paths.folder.join("tmp_export.mp4");
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let sink = FfmpegFrameSink::new_hq(tmp_str, layout.out_w, layout.out_h, OUT_FPS as f64)
        .context("create encode sink")?;
    // Encode on a worker thread so piping each 4K frame to ffmpeg overlaps with
    // compositing the next one (GPU compositor and encoder then run concurrently).
    let (tx, rx) = std::sync::mpsc::sync_channel::<Frame>(4);
    let encoder = std::thread::spawn(move || -> Result<()> {
        let mut sink = sink;
        for frame in rx { sink.push(&frame).context("encode push")?; }
        Box::new(sink).finish().context("finish encoder")?;
        Ok(())
    });

    let mut screen_buf = vec![0u8; screen_bytes];
    let mut wc_buf = vec![0u8; wc_bytes];
    let mut last_pct = u8::MAX;
    let mut cap_idx = 0usize;
    let mut have = screen_dec.read_frame(&mut screen_buf)?;

    for k in 0..=total_out {
        let t = video_start + k * 1000 / OUT_FPS;
        // Advance to the captured frame active at output time `t`.
        while have && cap_idx + 1 < tl.frames.len() && tl.frames[cap_idx + 1] <= t {
            have = screen_dec.read_frame(&mut screen_buf)?;
            if have { cap_idx += 1; }
        }
        let webcam = read_webcam(&mut webcam_dec, &mut wc_buf, size)?;
        let ev_t = (t.saturating_sub(tl.events_ms)) as u32;
        let scene = track.scene_at(ev_t);
        let cur = to_panel(cursor.at(ev_t), sw, sh, scene.screen.rect);
        let mut cam = sim.step(ev_t, cur, &regions, &cfg);
        if scene.screen.alpha < 0.5 {
            // No screen panel (CameraOnly): zoom is a no-op.
            cam = Camera { cx: layout.out_w as f32 / 2.0, cy: layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        let mut out = compositor.composite(&screen_buf, sw, sh, webcam, cam, &bg, &layout, &scene);
        crate::export::fxdraw::overlay(&mut out, layout.out_w, layout.out_h, &scene, cam, cur,
            &log.events, sw, sh, ev_t, &settings.clickfx, &actions, &settings.hotkeys);
        if tx.send(Frame { width: layout.out_w, height: layout.out_h, bgra: out, ts: Timestamp(t) }).is_err() {
            break; // encoder thread exited early; its error surfaces at join below
        }
        let pct = ((k * 100 / total_out).min(100)) as u8;
        if pct != last_pct {
            on_progress(pct);
            last_pct = pct;
        }
    }

    drop(tx);
    drop(screen_dec);
    drop(webcam_dec);
    encoder.join().map_err(|_| anyhow!("encoder thread panicked"))??;
    // Shift audio so its start lines up with the first video frame.
    let shift = |a: Option<u64>| a.map(|m| m as i64 - video_start as i64).unwrap_or(0);
    // Manual mic-sync nudge (cancels mic device input latency); system is loopback.
    let mic_shift = shift(tl.mic_ms) + settings.audio_offset_ms as i64;
    mux(&tmp, paths, mic_shift, shift(tl.system_ms))?;
    Ok(())
}

/// GPU compositor if a GPU is present and constructs, else CPU.
fn select_compositor(layout: &Layout) -> Box<dyn Compositor> {
    if gpu::gpu_available() {
        if let Some(c) = GpuCompositor::new(layout.out_w, layout.out_h) {
            return Box::new(c);
        }
    }
    Box::new(CpuCompositor)
}

/// Read one webcam frame; on EOF drop the decoder so later frames have none.
fn read_webcam<'a>(
    dec: &mut Option<RawDecoder>,
    buf: &'a mut [u8],
    size: u32,
) -> Result<Option<(&'a [u8], u32, u32)>> {
    let still = match dec {
        Some(d) => d.read_frame(buf)?,
        None => false,
    };
    if !still {
        *dec = None;
        return Ok(None);
    }
    Ok(Some((&*buf, size, size)))
}
