use super::exporter_report;
use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::bg_pipe::BgPipe;
use crate::export::pipeline::plan_walk::PlanCursor;
use crate::export::pipeline::{ScreenPipe, WebcamPipe};
use crate::export::render::FrameRenderer;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

pub(super) struct Pipes {
    pub spipe: ScreenPipe,
    pub wpipe: Option<WebcamPipe>,
    pub bgpipe: Option<BgPipe>,
    pub last_webcam: Option<(Vec<u8>, u32, u32)>,
    pub wc_fail: Option<String>,
    pub wc_frames: u64,
    pub held: Option<(usize, Vec<u8>)>,
}

#[derive(Default)]
pub(super) struct Timing {
    pub dec: u128,
    pub comp: u128,
    pub send: u128,
}

impl Pipes {
    fn advance(&mut self, video: &Path) -> Result<()> {
        self.spipe.next()?.ok_or_else(|| {
            anyhow!(
                "screen decode produced no frames from {:?} - the recording's video is unreadable",
                video
            )
        })?;
        if let Some(w) = &mut self.wpipe {
            match w.next() {
                Ok(Some(next)) => {
                    if let Some((old, _, _)) = self.last_webcam.take() {
                        w.recycle(old);
                    }
                    self.last_webcam = Some(next);
                    self.wc_frames += 1;
                }
                Ok(None) => {}
                Err(e) => {
                    if self.wc_fail.is_none() {
                        self.wc_fail = Some(e.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    fn feed_bg(&mut self, r: &mut FrameRenderer) {
        if self.bgpipe.as_mut().is_some_and(|b| !b.feed(r)) {
            if let Some(e) = self.bgpipe.take().and_then(|b| b.join().err()) {
                eprintln!("[EXPORT] background video ended early ({e}) - frozen on its last frame from here on");
            }
        }
    }
}

pub(super) struct Clock {
    pub video_start: u64,
    pub out_fps: u64,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    r: &mut FrameRenderer,
    pipes: &mut Pipes,
    plan: &[u64],
    video: &Path,
    clock: &Clock,
    out_dims: (u32, u32),
    out_pool: &BufPool,
    tx: &SyncSender<Frame>,
    on_progress: &impl Fn(u8),
) -> Result<(u64, Timing)> {
    let Clock {
        video_start,
        out_fps,
    } = *clock;
    let ms = |k: u64| k * 1000 / out_fps;
    let (mut timing, start) = (Timing::default(), Instant::now());
    let (mut cursor, total, mut last_pct, mut sent) = (
        PlanCursor::new(plan.to_vec()),
        plan.len() as u64,
        u8::MAX,
        0u64,
    );
    let mut failed: Option<anyhow::Error> = None;
    r.walk_plan(
        video_start,
        out_fps,
        plan,
        plan.len(),
        1000.0 / out_fps as f32,
        |r, j, _k, pose| {
            let step = cursor
                .next()
                .expect("the plan cursor and the walk cover the same frames");
            let d0 = Instant::now();
            for _ in 0..step.decodes_needed {
                if let Err(e) = pipes.advance(video) {
                    failed = Some(e);
                    return false;
                }
            }
            pipes.feed_bg(r);
            timing.dec += d0.elapsed().as_micros();
            let c0 = Instant::now();
            let mut out = out_pool.take();
            let Some(screen) = pipes.spipe.held() else {
                failed = Some(anyhow!("no screen frame held at output frame {j}"));
                return false;
            };
            let wc_ref = pipes
                .last_webcam
                .as_ref()
                .map(|(b, w, h)| (b.as_slice(), *w, *h));
            let prev = pose
                .mix
                .as_ref()
                .and_then(|m| pipes.held.as_ref().filter(|(i, _)| *i == m.prev_span))
                .map(|(_, b)| b.as_slice());
            let to_hold = pose.hold.map(|i| (i, screen.to_vec()));
            r.composite_at(pose, screen, prev, wc_ref, &mut out);
            if let Some(h) = to_hold {
                pipes.held = Some(h);
            }
            timing.comp += c0.elapsed().as_micros();
            let s0 = Instant::now();
            let ok = tx
                .send(Frame {
                    width: out_dims.0,
                    height: out_dims.1,
                    bgra: out,
                    ts: Timestamp(video_start + ms(j as u64)),
                })
                .is_ok();
            timing.send += s0.elapsed().as_micros();
            if !ok {
                return false;
            }
            sent += 1;
            last_pct =
                exporter_report::tick_progress(j as u64 + 1, total, last_pct, start, on_progress);
            true
        },
    );
    match failed {
        Some(e) => Err(e),
        None => Ok((sent, timing)),
    }
}
