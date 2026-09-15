use crate::export::camera::ease;
use crate::export::remap::TimeMap;
use crate::export::scene::layout::LayoutTrack;
use crate::export::scene::Scene;
use crate::export::types::{Easing, RectF};
use crate::session::sync::DisplaySwitch;

pub const SWITCH_MS: u32 = 350;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceSpan {
    pub start_ms: u32,
    pub src: RectF,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpanMix {
    pub prev_src: RectF,
    pub alpha: f32,
    pub prev_span: usize,
    pub hold_ms: u32,
}

pub fn hold_tick(start_ms: u32, fps: u64) -> u32 {
    let j = (u64::from(start_ms) * fps).saturating_sub(1) / 1000;
    (j * 1000 / fps) as u32
}

pub fn spans_of(
    switches: &[DisplaySwitch],
    frames: &[u64],
    canvas: (u32, u32),
    video_start: u64,
    map: &TimeMap,
) -> Vec<SourceSpan> {
    let full = crate::export::coordmap::full_src(canvas.0, canvas.1);
    let mut out = vec![SourceSpan {
        start_ms: 0,
        src: full,
    }];
    for s in switches {
        let at = frames
            .get(frames.partition_point(|&f| f < s.at_ms))
            .copied()
            .unwrap_or(s.at_ms);
        let clip = at.saturating_sub(video_start).min(u64::from(u32::MAX)) as u32;
        let start_ms = map.out_of(clip);
        let src = if s.w == 0 || s.h == 0 {
            full
        } else {
            fitted_rect((s.w, s.h), canvas)
        };
        match out.last_mut() {
            Some(p) if p.start_ms >= start_ms => {
                *p = SourceSpan {
                    start_ms: p.start_ms,
                    src,
                }
            }
            _ => out.push(SourceSpan { start_ms, src }),
        }
    }
    out
}

pub fn spans_for(
    paths: &crate::session::paths::ProjectPaths,
    canvas: (u32, u32),
    video_start: u64,
    map: &TimeMap,
) -> Vec<SourceSpan> {
    let log = crate::session::sync::SyncLog::load(&paths.sync()).unwrap_or_default();
    spans_of(&log.display_switches, &log.frames, canvas, video_start, map)
}

fn fitted_rect(src: (u32, u32), canvas: (u32, u32)) -> RectF {
    let (l, t, r, b) = crate::session::record::frame_fit::letterbox(src, canvas);
    RectF {
        x: l as f32,
        y: t as f32,
        w: (r - l) as f32,
        h: (b - t) as f32,
    }
}

pub struct SpanTrack {
    spans: Vec<SourceSpan>,
    tracks: Vec<LayoutTrack>,
    transition_ms: u32,
}

impl SpanTrack {
    pub fn build(
        spans: Vec<SourceSpan>,
        mut track_for: impl FnMut(u32, u32) -> LayoutTrack,
    ) -> Self {
        let tracks = spans
            .iter()
            .map(|s| track_for(s.src.w.max(1.0) as u32, s.src.h.max(1.0) as u32))
            .collect();
        Self {
            spans,
            tracks,
            transition_ms: SWITCH_MS,
        }
    }

    pub fn spans(&self) -> &[SourceSpan] {
        &self.spans
    }

    fn idx(&self, t: u32) -> usize {
        self.spans
            .iter()
            .rposition(|s| t >= s.start_ms)
            .unwrap_or(0)
    }

    fn raw(&self, i: usize, t: u32) -> Scene {
        self.tracks[i].scene_at(t).with_src(self.spans[i].src)
    }

    pub fn scene_at(&self, t: u32) -> Scene {
        self.frame_at(t, 60).0
    }

    pub fn frame_at(&self, t: u32, fps: u64) -> (Scene, Option<SpanMix>, Option<usize>) {
        let i = self.idx(t);
        let scene = self.raw(i, t);
        let hold = self
            .spans
            .get(i + 1)
            .filter(|n| t == hold_tick(n.start_ms, fps))
            .map(|_| i);
        let start = self.spans[i].start_ms;
        let elapsed = t.saturating_sub(start);
        if i == 0 || self.transition_ms == 0 || elapsed >= self.transition_ms {
            return (scene, None, hold);
        }
        let f = ease(Easing::Smooth, elapsed as f32 / self.transition_ms as f32);
        let from = self.raw(i - 1, t);
        let mix = SpanMix {
            prev_src: self.spans[i - 1].src,
            alpha: f,
            prev_span: i - 1,
            hold_ms: hold_tick(start, fps),
        };
        (Scene::lerp(&from, &scene, f), Some(mix), hold)
    }
}

#[cfg(test)]
#[path = "spans_tests.rs"]
mod tests;
