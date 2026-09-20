use crate::edit::clip::Clip;
use crate::export::camera::ease;
use crate::export::remap::TimeMap;
use crate::export::types::Easing;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipMix {
    pub prev_clip: usize,
    pub prev_out_ms: u32,
    pub alpha: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipDissolve {
    pub out_start_ms: u32,
    pub prev_clip: usize,
    pub dur_ms: u32,
}

pub struct ClipMixTrack {
    clips: Vec<Clip>,
    list: Vec<ClipDissolve>,
    easing: Easing,
}

impl ClipMixTrack {
    pub fn new(easing: Easing) -> Self {
        ClipMixTrack {
            clips: Vec::new(),
            list: Vec::new(),
            easing,
        }
    }

    pub fn set_clips(&mut self, clips: Vec<Clip>) {
        self.clips = clips;
        self.list.clear();
    }

    pub fn resolve(&mut self, map: &TimeMap, fps: u64) {
        self.list.clear();
        let spans = map.clip_spans(fps);
        for w in spans.windows(2) {
            let want = self.clips.get(w[1].clip).map_or(0, |c| c.transition_in_ms);
            let dur_ms = want.min((w[1].plan_len as u64 * 1000 / fps) as u32);
            if dur_ms == 0 {
                continue;
            }
            self.list.push(ClipDissolve {
                out_start_ms: (w[1].plan_start as u64 * 1000 / fps) as u32,
                prev_clip: w[0].clip,
                dur_ms,
            });
        }
    }

    pub fn dissolves(&self) -> &[ClipDissolve] {
        &self.list
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn at(&self, out_t: u32) -> Option<ClipMix> {
        let d = self
            .list
            .iter()
            .find(|d| out_t >= d.out_start_ms && out_t < d.out_start_ms + d.dur_ms)?;
        let p = (out_t - d.out_start_ms) as f32 / d.dur_ms as f32;
        Some(ClipMix {
            prev_clip: d.prev_clip,
            prev_out_ms: d.out_start_ms.saturating_sub(1),
            alpha: ease(self.easing, p),
        })
    }
}

#[cfg(test)]
#[path = "clipmix_tests.rs"]
mod tests;
