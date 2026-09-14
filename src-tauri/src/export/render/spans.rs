//! Source spans: which part of the recorded canvas the screen panel shows over time.
//!
//! A mid-take display switch keeps ONE encoder canvas (the first display's size) and fits every
//! later frame into it (`session::record::frame_fit::letterbox`), so the file itself carries baked
//! black bars from the switch onwards. The render undoes that: the take becomes a list of spans,
//! each `[start_ms (output clock), src_rect (source px)]`, and the screen panel shows exactly
//! `src` - the whole canvas before the first switch, the fitted rect of the switched-to display
//! after it. The panel therefore takes each span's own aspect (a 16:10 display gets a 16:10 panel
//! with the project background beside it), which is why every span gets its own `LayoutTrack`:
//! `scene::resolve` shapes the screen panel from the source size it is handed.
//!
//! The switch itself eases: over `SWITCH_MS` the panel rect blends from the old span's scene to
//! the new one's through `Scene::lerp` - the very machinery a layout transition uses - while the
//! two PICTURES cross-dissolve (`SpanMix`, applied by `render::screen_mix` before compositing).
use crate::export::camera::ease;
use crate::export::remap::TimeMap;
use crate::export::scene::layout::LayoutTrack;
use crate::export::scene::Scene;
use crate::export::types::{Easing, RectF};
use crate::session::sync::DisplaySwitch;

/// Display-switch transition length (ms). The same 350 ms a layout segment gets by default
/// (`render_edit::TRANSITION_MS`), on the same `Easing::Smooth` curve.
pub const SWITCH_MS: u32 = 350;

/// One source span: from `start_ms` on the output clock, the screen panel shows `src` of the
/// recorded canvas. The first span always starts at 0 with the whole canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceSpan { pub start_ms: u32, pub src: RectF }

/// A display switch in flight at one instant: the picture to blend FROM and how much of the new
/// one is mixed in. `alpha` is the eased 0..1 weight of the NEW span's picture (0 at the switch
/// instant, 1 when the transition ends) - the same curve the panel rect eases on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpanMix { pub prev_src: RectF, pub alpha: f32, pub prev_span: usize, pub hold_ms: u32 }

/// The output tick whose decoded screen frame a switch at `start_ms` dissolves FROM: the last
/// output frame before the switch. Both the exporter (which latches that frame as it streams past)
/// and the one-shot preview (which seek-decodes it) derive it from here, so the ghost image in a
/// mid-transition preview frame is the very frame the export blends.
pub fn hold_tick(start_ms: u32, fps: u64) -> u32 {
    let j = (u64::from(start_ms) * fps).saturating_sub(1) / 1000; // largest j with j*1000/fps < start
    (j * 1000 / fps) as u32
}

/// The span table for a take. `switches` are `sync.json`'s entries and `frames` its frame times
/// (both on the recording clock, `frames` ascending); `canvas` is the video's own size. A switch
/// whose geometry was never recorded (`w`/`h` 0 - a log written before `DisplaySwitch` carried
/// them) spans the full canvas, i.e. today's behavior.
pub fn spans_of(switches: &[DisplaySwitch], frames: &[u64], canvas: (u32, u32), video_start: u64, map: &TimeMap) -> Vec<SourceSpan> {
    let full = crate::export::coordmap::full_src(canvas.0, canvas.1);
    let mut out = vec![SourceSpan { start_ms: 0, src: full }];
    for s in switches {
        // The span opens on the first frame the NEW capture delivered, not on the stamp: the stamp
        // precedes the restart (`switch_display`) and the first new frame lands 50 to 300 ms after
        // it. Opened at the stamp, the span would crop and dissolve the OLD display's held frame
        // and then pop to the new picture mid-fade. No frame after the stamp: the stamp stands.
        let at = frames.get(frames.partition_point(|&f| f < s.at_ms)).copied().unwrap_or(s.at_ms);
        // Recording clock -> clip clock -> output clock, exactly like every other recorded region.
        let clip = at.saturating_sub(video_start).min(u64::from(u32::MAX)) as u32;
        let start_ms = map.out_of(clip);
        let src = if s.w == 0 || s.h == 0 { full } else { fitted_rect((s.w, s.h), canvas) };
        match out.last_mut() {
            // Two switches inside the same output millisecond (or one landing at 0, or inside a
            // cut, where `out_of` collapses a whole range onto one instant): the later wins.
            Some(p) if p.start_ms >= start_ms => *p = SourceSpan { start_ms: p.start_ms, src },
            _ => out.push(SourceSpan { start_ms, src }),
        }
    }
    out
}

/// The span table for the recording at `paths`, reading `sync.json`'s `display_switches`. A
/// missing, unreadable or switch-free log is a take that never switched: one full-canvas span,
/// which is what makes such a take render byte-identically to before spans existed.
pub fn spans_for(paths: &crate::session::paths::ProjectPaths, canvas: (u32, u32),
                 video_start: u64, map: &TimeMap) -> Vec<SourceSpan> {
    let log = crate::session::sync::SyncLog::load(&paths.sync()).unwrap_or_default();
    spans_of(&log.display_switches, &log.frames, canvas, video_start, map)
}

/// Where a `src`-sized capture lands inside the `canvas`-sized encoder frame, as a `RectF` - the
/// EXACT rect `frame_fit::letterbox` placed it at, so the crop lands on the fitted picture to the
/// pixel rather than on a re-derived approximation of it.
fn fitted_rect(src: (u32, u32), canvas: (u32, u32)) -> RectF {
    let (l, t, r, b) = crate::session::record::frame_fit::letterbox(src, canvas);
    RectF { x: l as f32, y: t as f32, w: (r - l) as f32, h: (b - t) as f32 }
}

/// The take's spans plus one `LayoutTrack` per span (built at that span's own source size), which
/// together answer "what does this frame look like". A take with no switch has exactly one span
/// and one track, and every method below degenerates to that track's own answer.
pub struct SpanTrack { spans: Vec<SourceSpan>, tracks: Vec<LayoutTrack>, transition_ms: u32 }

impl SpanTrack {
    /// Build from a span table and a per-span `LayoutTrack` factory (`track_for(sw, sh)` - the
    /// span's own source size, which is what shapes the screen panel).
    pub fn build(spans: Vec<SourceSpan>, mut track_for: impl FnMut(u32, u32) -> LayoutTrack) -> Self {
        let tracks = spans.iter()
            .map(|s| track_for(s.src.w.max(1.0) as u32, s.src.h.max(1.0) as u32))
            .collect();
        Self { spans, tracks, transition_ms: SWITCH_MS }
    }

    pub fn spans(&self) -> &[SourceSpan] { &self.spans }

    /// Index of the span active at `t`: the last one that has started.
    fn idx(&self, t: u32) -> usize {
        self.spans.iter().rposition(|s| t >= s.start_ms).unwrap_or(0)
    }

    /// The span's own scene at `t`, carrying that span's source rect.
    fn raw(&self, i: usize, t: u32) -> Scene {
        self.tracks[i].scene_at(t).with_src(self.spans[i].src)
    }

    /// The scene at `t` with any switch transition already blended in - what every consumer that
    /// only needs geometry (zoom anchoring, the preview's per-segment rects) asks for. The frame
    /// rate is irrelevant here: it only ever decides WHICH frame to latch, which this drops.
    pub fn scene_at(&self, t: u32) -> Scene { self.frame_at(t, 60).0 }

    /// Everything one output frame needs: the blended scene, the display-switch dissolve in flight
    /// (if any), and - when this is the last output frame before the NEXT switch - the index of the
    /// span whose decoded screen frame the caller must latch for that dissolve.
    ///
    /// Only ONE switch is ever in flight: a second switch inside the first's 350 ms takes over
    /// from the (already settled) span before it, exactly as `LayoutTrack` lets a segment's entry
    /// win over its predecessor's exit rather than double-blending.
    pub fn frame_at(&self, t: u32, fps: u64) -> (Scene, Option<SpanMix>, Option<usize>) {
        let i = self.idx(t);
        let scene = self.raw(i, t);
        let hold = self.spans.get(i + 1)
            .filter(|n| t == hold_tick(n.start_ms, fps))
            .map(|_| i);
        let start = self.spans[i].start_ms;
        let elapsed = t.saturating_sub(start);
        if i == 0 || self.transition_ms == 0 || elapsed >= self.transition_ms {
            return (scene, None, hold);
        }
        let f = ease(Easing::Smooth, elapsed as f32 / self.transition_ms as f32);
        let from = self.raw(i - 1, t);
        let mix = SpanMix { prev_src: self.spans[i - 1].src, alpha: f, prev_span: i - 1,
            hold_ms: hold_tick(start, fps) };
        (Scene::lerp(&from, &scene, f), Some(mix), hold)
    }
}

#[cfg(test)]
#[path = "spans_tests.rs"]
mod tests;
