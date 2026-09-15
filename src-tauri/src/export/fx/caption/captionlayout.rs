use crate::edit::captions::Caption;
use crate::settings::captions::{CaptionAnim, CaptionPos, CaptionStyle};

pub const ADV: f32 = 0.52;

pub const LINE_H: f32 = 1.32;

pub const PAD_X: f32 = 0.60;
pub const PAD_Y: f32 = 0.34;

pub const MARGIN: f32 = 0.075;

pub const RISE_LINES: f32 = 0.5;

pub const POP_FROM: f32 = 0.92;

pub const MAX_CHARS: usize = 42;
pub const MAX_LINES: usize = 2;

pub struct LaidCaption {
    pub lines: Vec<String>,
    pub font_px: f32,
    pub line_h: f32,
    pub pill: [f32; 4],
    pub baselines: Vec<f32>,
    pub hi: Option<usize>,
    pub alpha: f32,
    pub rise: f32,
    pub scale: f32,
    pub words_shown: Option<usize>,
    pub word_alpha: f32,
}

pub fn ease_out(p: f32) -> f32 {
    let q = 1.0 - p.clamp(0.0, 1.0);
    1.0 - q * q * q
}

pub fn ramp(dt: f32, ms: f32) -> f32 {
    if ms <= 0.0 {
        1.0
    } else {
        (dt / ms).clamp(0.0, 1.0)
    }
}

pub fn wrap_lines(text: &str, max: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in text.split_whitespace() {
        match out.last_mut() {
            Some(l) if l.chars().count() + 1 + w.chars().count() <= max => {
                l.push(' ');
                l.push_str(w);
            }
            _ => out.push(w.to_string()),
        }
    }
    out
}

pub fn caption_at(caps: &[Caption], t_ms: u32) -> Option<&Caption> {
    caps.iter().find(|c| c.start_ms <= t_ms && t_ms < c.end_ms)
}

pub fn word_span(cap: &Caption, i: usize) -> Option<(usize, usize)> {
    let w = cap.words.get(i)?;
    let start: usize = cap.words[..i]
        .iter()
        .map(|x| x.text.chars().count() + 1)
        .sum();
    Some((start, start + w.text.chars().count()))
}

pub fn layout(cap: &Caption, style: &CaptionStyle, ow: u32, oh: u32, t_ms: u32) -> LaidCaption {
    let (ow, oh) = (ow as f32, oh as f32);
    let (t, ms) = (t_ms as f32, style.animation_ms as f32);
    let anim = style.animation;
    let p_in = ramp(t - cap.start_ms as f32, ms);
    // INVARIANT: outside the span every kind is fully transparent, `none` included.
    let alpha = if t_ms < cap.start_ms || t_ms >= cap.end_ms {
        0.0
    } else if anim == CaptionAnim::None {
        1.0
    } else {
        p_in.min(ramp(cap.end_ms as f32 - t, ms))
    };
    let scale = if anim == CaptionAnim::Pop {
        POP_FROM + (1.0 - POP_FROM) * ease_out(p_in)
    } else {
        1.0
    };
    let font_px = (oh * style.height_frac()).max(8.0) * scale;
    let line_h = font_px * LINE_H;
    let rise = if anim == CaptionAnim::Rise {
        line_h * RISE_LINES * (1.0 - ease_out(p_in))
    } else {
        0.0
    };
    let hi = style
        .highlight
        .then(|| cap.words.iter().rposition(|w| w.start_ms <= t_ms))
        .flatten();
    let (words_shown, word_alpha) = reveal(cap, anim, t, ms, t_ms);
    let mut lines = wrap_lines(cap.text.trim(), MAX_CHARS);
    lines.truncate(MAX_LINES);
    let mut l = LaidCaption {
        lines,
        font_px,
        line_h,
        pill: [0.0; 4],
        baselines: Vec::new(),
        hi,
        alpha,
        rise,
        scale,
        words_shown,
        word_alpha,
    };
    if l.lines.is_empty() {
        return l;
    }
    let widest = l.lines.iter().map(|x| x.chars().count()).max().unwrap_or(0) as f32;
    let pw = widest * font_px * ADV + 2.0 * font_px * PAD_X;
    let ph = l.lines.len() as f32 * line_h + 2.0 * font_px * PAD_Y;
    let px = (ow - pw) / 2.0;
    let py = rise
        + match style.position {
            CaptionPos::Bottom => oh - oh * MARGIN - ph,
            CaptionPos::Top => oh * MARGIN,
        };
    l.baselines = (0..l.lines.len())
        .map(|i| py + font_px * PAD_Y + line_h * i as f32 + font_px)
        .collect();
    l.pill = [px, py, pw, ph];
    l
}

fn reveal(cap: &Caption, anim: CaptionAnim, t: f32, ms: f32, t_ms: u32) -> (Option<usize>, f32) {
    if anim != CaptionAnim::Words || cap.words.is_empty() {
        return (None, 1.0);
    }
    let n = cap.words.iter().filter(|w| w.start_ms <= t_ms).count();
    let a = match n.checked_sub(1) {
        Some(i) => ramp(t - cap.words[i].start_ms as f32, ms),
        None => 1.0,
    };
    (Some(n), a)
}

#[cfg(test)]
#[path = "captionlayout_tests.rs"]
mod tests;
