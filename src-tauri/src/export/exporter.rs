// Exporter: decode the screen + webcam, composite each frame with auto-zoom,
// encode to a temp mp4, then mux audio into final.mp4.
use anyhow::{anyhow, Context, Result};

use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::encode::ffmpeg_encoder::FfmpegFrameSink;
use crate::encode::frame_sink::FrameSink;
use crate::events::model::{EventLog, MouseEvent};
use crate::export::audio_mux::mux;
use crate::export::background;
use crate::export::camera::CameraSim;
use crate::export::compositor::{Compositor, CpuCompositor};
use crate::export::coordmap::to_frame;
use crate::export::ffio::{probe_dims, probe_duration, RawDecoder};
use crate::export::types::{Background, FramePoint, Layout, OverlayLayout, ZoomConfig};
use crate::export::{autozoom, gpu, gpu_compositor::GpuCompositor};
use crate::session::paths::ProjectPaths;

/// Render a recording into `paths.folder/final.mp4`. `fps` sets the constant
/// output rate; `on_progress` receives 0..=100 as frames are encoded.
pub fn export(paths: &ProjectPaths, fps: u32, on_progress: impl Fn(u8)) -> Result<()> {
    let log = EventLog::load(&paths.events()).context("load events.json")?;
    let cfg = ZoomConfig::default();
    let regions = autozoom::generate(&log.events, &log.screen, &cfg);

    let video = paths.video();
    let (sw, sh) = probe_dims(&video)?;
    let duration = probe_duration(&video)?;
    let layout = Layout::default();
    let total_frames = ((duration * fps as f64).round() as u64).max(1);
    let bg = background::render(&Background::default(), layout.out_w, layout.out_h);

    let compositor = select_compositor(&layout);
    let mut sim = CameraSim::new(sw, sh);
    let mut cursor = Cursor::new(&log.events, &log.screen);

    // Decoders: screen drives the loop length; webcam stops early if shorter.
    let screen_bytes = (sw * sh * 4) as usize;
    let mut screen_dec = RawDecoder::spawn(&video, fps, None, screen_bytes)?;
    let ov = OverlayLayout::default();
    let size = ov.size_px;
    let wc_bytes = (size * size * 4) as usize;
    let mut webcam_dec = if paths.webcam().exists() {
        Some(RawDecoder::spawn(&paths.webcam(), fps, Some(size), wc_bytes)?)
    } else {
        None
    };

    let tmp = paths.folder.join("tmp_export.mp4");
    let tmp_str = tmp.to_str().ok_or_else(|| anyhow!("non-utf8 tmp path"))?;
    let mut sink = FfmpegFrameSink::new(tmp_str, layout.out_w, layout.out_h, fps)
        .context("create encode sink")?;

    let mut screen_buf = vec![0u8; screen_bytes];
    let mut wc_buf = vec![0u8; wc_bytes];
    let mut last_pct = u8::MAX;

    for i in 0u64.. {
        if !screen_dec.read_frame(&mut screen_buf)? {
            break;
        }
        let webcam = read_webcam(&mut webcam_dec, &mut wc_buf, size)?;

        let t_ms = (i * 1000 / fps as u64) as u32;
        let cam = sim.step(t_ms, cursor.at(t_ms), &regions, &cfg);
        let out = compositor.composite(
            &screen_buf, sw, sh, webcam, cam, &bg, &layout, &ov,
        );
        sink.push(&Frame {
            width: layout.out_w,
            height: layout.out_h,
            bgra: out,
            ts: Timestamp(t_ms as u64),
        })?;

        let pct = (((i + 1) * 100 / total_frames).min(100)) as u8;
        if pct != last_pct {
            on_progress(pct);
            last_pct = pct;
        }
    }

    drop(screen_dec);
    drop(webcam_dec);
    Box::new(sink).finish().context("finish encoder")?;
    mux(&tmp, paths)?;
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

/// Monotonic cursor lookup: the last event with `t <= t_ms`, in frame space.
struct Cursor<'a> {
    events: &'a [MouseEvent],
    screen: &'a crate::events::model::ScreenInfo,
    idx: usize,
}

impl<'a> Cursor<'a> {
    fn new(events: &'a [MouseEvent], screen: &'a crate::events::model::ScreenInfo) -> Self {
        Self { events, screen, idx: 0 }
    }

    /// Advance the index to the last event at or before `t_ms` (no rescans).
    fn at(&mut self, t_ms: u32) -> FramePoint {
        while self.idx + 1 < self.events.len() && self.events[self.idx + 1].t <= t_ms {
            self.idx += 1;
        }
        let cur = self.events.get(self.idx);
        match cur {
            Some(e) if e.t <= t_ms => to_frame(self.screen, e.x, e.y),
            _ => FramePoint {
                x: self.screen.w as i32 / 2,
                y: self.screen.h as i32 / 2,
            },
        }
    }
}
