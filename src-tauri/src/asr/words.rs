use crate::edit::captions::CaptionWord;

#[derive(Clone, Debug, PartialEq)]
pub struct RawToken {
    pub text: String,
    pub t0_cs: i64,
    pub t1_cs: i64,
}

pub fn is_special(t: &str) -> bool {
    t.starts_with("[_") || t.starts_with("<|")
}

fn cs_to_ms(cs: i64) -> u32 {
    cs.max(0).saturating_mul(10) as u32
}

pub fn words_from_tokens(tokens: &[RawToken]) -> Vec<CaptionWord> {
    let mut out: Vec<CaptionWord> = Vec::new();
    for t in tokens {
        if is_special(&t.text) {
            continue;
        }
        let piece = t.text.trim();
        if piece.is_empty() {
            continue;
        }
        let (start_ms, end_ms) = (cs_to_ms(t.t0_cs), cs_to_ms(t.t1_cs));
        match out.last_mut() {
            Some(w) if !t.text.starts_with(' ') => {
                w.text.push_str(piece);
                w.end_ms = w.end_ms.max(end_ms);
            }
            _ => out.push(CaptionWord {
                start_ms,
                end_ms,
                text: piece.to_string(),
            }),
        }
    }
    for i in 0..out.len() {
        if out[i].end_ms > out[i].start_ms {
            continue;
        }
        let next = out.get(i + 1).map(|w| w.start_ms);
        out[i].end_ms = match next {
            Some(n) if n > out[i].start_ms + 1 => n - 1,
            _ => out[i].start_ms + 1,
        };
    }
    out
}

#[cfg(test)]
#[path = "words_tests.rs"]
mod tests;
