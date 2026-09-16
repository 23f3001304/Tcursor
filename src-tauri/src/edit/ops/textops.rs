use crate::edit::model::EditDoc;
use crate::edit::ops::api::EditOp;
use crate::edit::ops::ids::next_text_id;
use crate::edit::ops::region::{clamp_order, dur_bound, valid_easing};
use crate::edit::text::{
    default_anim_ms, default_text_easing, TextAnchor, TextAnim, TextItem, TextKind, TextSize,
};

pub const TEXT_STYLES: [&str; 4] = ["clean", "plate", "accent", "bar"];
pub const MAX_TEXT_CHARS: usize = 200;
const MAX_ANIM_MS: u32 = 4000;

pub fn valid_text_style(s: &str) -> String {
    if TEXT_STYLES.contains(&s) {
        s.to_string()
    } else {
        "clean".into()
    }
}

fn tidy(s: &str) -> String {
    s.trim().chars().take(MAX_TEXT_CHARS).collect()
}

fn seed(
    kind: TextKind,
) -> (
    &'static str,
    Option<&'static str>,
    TextSize,
    TextAnchor,
    &'static str,
    TextAnim,
) {
    match kind {
        TextKind::Title => (
            "Your title",
            None,
            TextSize::L,
            TextAnchor::MidCenter,
            "clean",
            TextAnim::Fade,
        ),
        TextKind::LowerThird => (
            "Name",
            Some("Role"),
            TextSize::M,
            TextAnchor::BottomLeft,
            "bar",
            TextAnim::Fade,
        ),
        TextKind::Stat => (
            "128",
            Some("faster"),
            TextSize::Xl,
            TextAnchor::MidCenter,
            "clean",
            TextAnim::Fade,
        ),
        TextKind::Callout => (
            "A callout",
            None,
            TextSize::S,
            TextAnchor::BottomCenter,
            "plate",
            TextAnim::Typewriter,
        ),
    }
}

pub fn apply_text(doc: &mut EditDoc, op: EditOp) {
    let dur = dur_bound(doc);
    match op {
        EditOp::AddText {
            at_ms,
            dur_ms,
            kind,
        } => {
            let (text, sub, size, pos, style, anim_in) = seed(kind);
            let id = next_text_id(doc);
            doc.texts.push(TextItem {
                id,
                start_ms: at_ms.min(dur),
                end_ms: at_ms.saturating_add(dur_ms).min(dur),
                kind,
                text: text.into(),
                sub: sub.map(Into::into),
                style: style.into(),
                pos,
                offset: [0.0, 0.0],
                size,
                anim_in,
                anim_out: TextAnim::Fade,
                in_ms: default_anim_ms(),
                out_ms: default_anim_ms(),
                easing: default_text_easing(),
            });
        }
        EditOp::UpdateText {
            id,
            start_ms,
            end_ms,
            text,
            sub,
            kind,
            style,
            pos,
            offset,
            size,
            anim_in,
            anim_out,
            in_ms,
            out_ms,
            easing,
        } => {
            let Some(t) = doc.texts.iter_mut().find(|t| t.id == id) else {
                return;
            };
            if let Some(v) = start_ms {
                t.start_ms = v.min(dur);
            }
            if let Some(v) = end_ms {
                t.end_ms = v.min(dur);
            }
            clamp_order(&mut t.start_ms, &mut t.end_ms, start_ms.is_some());
            if let Some(v) = text {
                t.text = tidy(&v);
            }
            if let Some(v) = sub {
                t.sub = v.map(|s| tidy(&s));
            }
            if let Some(v) = kind {
                t.kind = v;
            }
            if let Some(v) = style {
                t.style = valid_text_style(&v);
            }
            if let Some(v) = pos {
                t.pos = v;
            }
            if let Some(o) = offset.filter(|o| o.iter().all(|c| c.is_finite())) {
                t.offset = [o[0].clamp(-0.5, 0.5), o[1].clamp(-0.5, 0.5)];
            }
            if let Some(v) = size {
                t.size = v;
            }
            if let Some(v) = anim_in {
                t.anim_in = v;
            }
            if let Some(v) = anim_out {
                t.anim_out = v;
            }
            if let Some(v) = in_ms {
                t.in_ms = v.min(MAX_ANIM_MS);
            }
            if let Some(v) = out_ms {
                t.out_ms = v.min(MAX_ANIM_MS);
            }
            if let Some(v) = easing {
                t.easing = valid_easing(&v);
            }
        }
        EditOp::RemoveText { id } => doc.texts.retain(|t| t.id != id),
        _ => {}
    }
}

#[cfg(test)]
#[path = "textops_tests.rs"]
mod tests;
