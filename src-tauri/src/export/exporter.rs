// Exporter: composite each frame with auto-zoom, encode to tmp mp4, mux audio.
// Frames are placed by real capture timestamp so variable-rate capture works.
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
use crate::actions::model::LayoutId;
use crate::export::types::{Background, Camera, Layout};
use crate::export::{autozoom, gpu, gpu_compositor::GpuCompositor};
use crate::session::paths::ProjectPaths;

/// Bundled default background (a macOS-style gradient wallpaper).
const BG_MESH: &[u8] = include_bytes!("../../assets/backgrounds/bg.jpg");
/// Bundled CC0 pointer sprite (114x174, white arrow, hotspot at top-left tip).
const POINTER_PNG: &[u8] = include_bytes!("../../assets/cursors/pointer.png");

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
    let track = crate::export::layout::LayoutTrack::new(
        &actions, &settings.appearance, layout.out_w, layout.out_h, sw, sh, TRANSITION_MS);

    // Click + manual hold-to-zoom regions; manual comes last so active hold wins.
    let typing = crate::events::typing::TypingLog::load(&paths.typing()).ms;
    let mut raw = if settings.zoom.enabled {
        autozoom::generate(&log.events, &log.screen, &cfg, &typing, settings.zoom.smart_hold)
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
    let fx_renderer = crate::export::fx_state::select_fx(layout.out_w, layout.out_h);
    let mut sim = CameraSim::new(layout.out_w, layout.out_h);
    let mut cursor = Cursor::new(&log.events, &log.screen);
    // Synthetic cursor: decode sprite + collect click timestamps (Enhanced only).
    let mut cprep = crate::export::cursordraw::prep(&settings.cursor, &log.events, POINTER_PNG);

    let screen_bytes = (sw * sh * 4) as usize;
    let mut screen_dec = RawDecoder::spawn(&video, 0.0, false, None, None, screen_bytes)?;
    // Decode the webcam at the largest camera panel any mode needs (cap to bound
    // memory; panels resize down). Defaults give 1920 -> capped to 1440 (today).
    let max_cam = [LayoutId::Screen, LayoutId::Camera, LayoutId::Presenter,
                   LayoutId::ScreenOnly, LayoutId::CameraOnly]
        .iter()
        .map(|&id| crate::settings::appearance::overlay_for(
            settings.appearance.for_id(id), layout.out_w, layout.out_h, true).size_px)
        .max().unwrap_or(420);
    let size = max_cam.min(1440).max(1);
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

    let export_start = std::time::Instant::now();
    let (mut t_dec, mut t_comp, mut t_send) = (0u128, 0u128, 0u128);
    for k in 0..=total_out {
        let t = video_start + k * 1000 / OUT_FPS;
        let d0 = std::time::Instant::now();
        // Advance to the captured frame active at output time `t`.
        while have && cap_idx + 1 < tl.frames.len() && tl.frames[cap_idx + 1] <= t {
            have = screen_dec.read_frame(&mut screen_buf)?;
            if have { cap_idx += 1; }
        }
        let webcam = read_webcam(&mut webcam_dec, &mut wc_buf, size)?;
        t_dec += d0.elapsed().as_micros();
        let ev_t = (t.saturating_sub(tl.events_ms)) as u32;
        let mut scene = track.scene_at(ev_t);
        let cur = to_panel(cursor.at(ev_t), sw, sh, scene.screen.rect);
        let mut cam = sim.step(ev_t, cur, &regions, &cfg);
        if scene.screen.alpha < 0.5 {
            // No screen panel (CameraOnly): zoom is a no-op.
            cam = Camera { cx: layout.out_w as f32 / 2.0, cy: layout.out_h as f32 / 2.0, scale: 1.0 };
        }
        // Auto camera shrink: keep the webcam out of the way while zoomed in. Skip when
        // the camera is already the dominant panel (camera-focused layouts).
        if settings.zoom.camera_shrink && scene.camera.rect.w < scene.screen.rect.w {
            scene.camera = crate::export::scene::shrink_camera(
                scene.camera, cam.scale, cfg.target_scale, settings.zoom.camera_shrink_min);
        }
        let c0 = std::time::Instant::now();
        let mut out = compositor.composite(&screen_buf, sw, sh, webcam, cam, &bg, &layout, &scene);
        crate::export::fx_state::render(&*fx_renderer, &mut out, layout.out_w, layout.out_h,
            &settings.clickfx, &log.events, &actions, &scene, cam, cur, sw, sh, ev_t, &settings.hotkeys);
        if let Some(cp) = &mut cprep {
            let pos = crate::export::coordmap::project(cur.x as f32, cur.y as f32, cam, layout.out_w, layout.out_h);
            crate::export::cursordraw::apply_enhanced(&mut out, layout.out_w, layout.out_h, &cp.sprite,
                pos, &mut cp.recent, 6, &cp.click_ms, ev_t, settings.cursor.size, settings.cursor.motion_blur, settings.cursor.click_bounce);
        }
        t_comp += c0.elapsed().as_micros();
        let s0 = std::time::Instant::now();
        if tx.send(Frame { width: layout.out_w, height: layout.out_h, bgra: out, ts: Timestamp(t) }).is_err() {
            break; // encoder thread exited early; its error surfaces at join below
        }
        t_send += s0.elapsed().as_micros();
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
    // TEMP perf diagnostic: per-stage breakdown -> %TEMP%/tcursor-export-timing.txt.
    let secs = export_start.elapsed().as_secs_f64().max(0.001);
    let _ = std::fs::write(std::env::temp_dir().join("tcursor-export-timing.txt"), format!(
        "frames={} total={:.2}s fps={:.1} decode={}ms composite={}ms encode_wait={}ms\n",
        total_out + 1, secs, (total_out + 1) as f64 / secs, t_dec / 1000, t_comp / 1000, t_send / 1000));
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
