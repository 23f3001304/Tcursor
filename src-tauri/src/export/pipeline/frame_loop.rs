use super::exporter_report;
use crate::capture::frame::Frame;
use crate::domain::time::Timestamp;
use crate::export::gpu::pool::BufPool;
use crate::export::pipeline::bg_pipe::BgPipe;
use crate::export::pipeline::plan_walk::{join_at, PlanCursor};
use crate::export::pipeline::{ScreenPipe, WebcamPipe};
use crate::export::remap::ClipSpan;
use crate::export::render::screen_mix;
use crate::export::render::FrameRenderer;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
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
    pub clip_held: Option<(usize, Vec<u8>)>,
    pub mix_scratch: Vec<u8>,
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

    fn latch_clip(&mut self, prev_clip: usize) {
        self.clip_held = self.spipe.held().map(|b| (prev_clip, b.to_vec()));
    }

    fn rewind(&mut self, d: &ClipDecode, clock: &Clock, first_k: u64) -> Result<()> {
        let seek = first_k * 1000 / clock.out_fps;
        let next = ScreenPipe::spawn(
            &d.video,
            d.screen_bytes,
            d.screen_crop,
            None,
            d.depth,
            clock.out_fps,
            Some(seek),
        )?;
        std::mem::replace(&mut self.spipe, next).join()?;
        self.rewind_webcam(d, clock, seek);
        Ok(())
    }

    fn rewind_webcam(&mut self, d: &ClipDecode, clock: &Clock, seek: u64) {
        let Some(old) = self.wpipe.take() else {
            return;
        };
        if let Some(p) = &d.webcam {
            match WebcamPipe::spawn(
                p,
                clock.video_start + seek,
                d.wc_dims,
                d.wc_bytes,
                d.depth,
                clock.out_fps,
            ) {
                Ok(w) => {
                    if let Some((buf, _, _)) = self.last_webcam.take() {
                        old.recycle(buf);
                    }
                    self.wpipe = Some(w);
                }
                Err(e) => {
                    self.wc_fail.get_or_insert_with(|| e.to_string());
                }
            }
        }
        if let Err(e) = old.join() {
            self.wc_fail.get_or_insert_with(|| e.to_string());
        }
    }
}

pub(super) struct Clock {
    pub video_start: u64,
    pub out_fps: u64,
}

pub(super) struct ClipDecode {
    pub video: PathBuf,
    pub webcam: Option<PathBuf>,
    pub screen_bytes: usize,
    pub screen_crop: Option<(u32, u32)>,
    pub wc_dims: (u32, u32),
    pub wc_bytes: usize,
    pub depth: usize,
    pub sw: u32,
    pub sh: u32,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    r: &mut FrameRenderer,
    pipes: &mut Pipes,
    plan: &[u64],
    spans: &[ClipSpan],
    dec: &ClipDecode,
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
            if let Some(join) = join_at(spans, plan, j) {
                pipes.latch_clip(join.prev_clip);
                if join.respawn {
                    if let Err(e) = pipes.rewind(dec, clock, join.first_k) {
                        failed = Some(e);
                        return false;
                    }
                    cursor.rebase(join.first_k);
                }
            }
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
            let Pipes {
                spipe,
                clip_held,
                mix_scratch,
                last_webcam,
                held,
                ..
            } = &mut *pipes;
            let Some(screen) = spipe.held() else {
                failed = Some(anyhow!("no screen frame held at output frame {j}"));
                return false;
            };
            let screen = match (pose.clip_mix, clip_held.as_ref()) {
                (Some(m), Some((c, prev))) if *c == m.prev_clip => {
                    screen_mix::blend_into(
                        mix_scratch,
                        screen,
                        prev,
                        dec.sw,
                        dec.sh,
                        pose.scene.src,
                        pose.scene.src,
                        m.alpha,
                    );
                    &mix_scratch[..]
                }
                _ => screen,
            };
            let wc_ref = last_webcam.as_ref().map(|(b, w, h)| (b.as_slice(), *w, *h));
            let prev = pose
                .mix
                .as_ref()
                .and_then(|m| held.as_ref().filter(|(i, _)| *i == m.prev_span))
                .map(|(_, b)| b.as_slice());
            let to_hold = pose.hold.map(|i| (i, screen.to_vec()));
            r.composite_at(pose, screen, prev, wc_ref, &mut out);
            if let Some(h) = to_hold {
                *held = Some(h);
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
