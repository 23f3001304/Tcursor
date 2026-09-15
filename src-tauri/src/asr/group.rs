use crate::edit::captions::{Caption, CaptionWord};

#[derive(Clone, Debug, PartialEq)]
pub struct GroupLimits {
    pub max_chars_per_line: usize,
    pub max_lines: usize,
    pub max_ms: u32,
    pub max_gap_ms: u32,
}
impl Default for GroupLimits {
    fn default() -> Self {
        Self {
            max_chars_per_line: 42,
            max_lines: 2,
            max_ms: 4500,
            max_gap_ms: 700,
        }
    }
}

pub fn wrap_lines(text: &str, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for w in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + w.chars().count() <= max => {
                l.push(' ');
                l.push_str(w);
            }
            _ => lines.push(w.to_string()),
        }
    }
    lines
}

fn ends_sentence(t: &str) -> bool {
    t.ends_with('.') || t.ends_with('?') || t.ends_with('!')
}

fn joined(words: &[CaptionWord]) -> String {
    words
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn close(caps: &mut Vec<Caption>, words: Vec<CaptionWord>) {
    let (Some(first), Some(last)) = (words.first(), words.last()) else {
        return;
    };
    caps.push(Caption {
        id: format!("c{}", caps.len()),
        start_ms: first.start_ms,
        end_ms: last.end_ms,
        text: joined(&words),
        words,
    });
}

pub fn group_words(words: &[CaptionWord], lim: &GroupLimits) -> Vec<Caption> {
    let mut caps: Vec<Caption> = Vec::new();
    let mut cur: Vec<CaptionWord> = Vec::new();
    for w in words {
        let breaks = match (cur.first(), cur.last()) {
            (Some(first), Some(prev)) => {
                w.end_ms.saturating_sub(first.start_ms) > lim.max_ms
                    || w.start_ms.saturating_sub(prev.end_ms) > lim.max_gap_ms
                    || (ends_sentence(&prev.text) && cur.len() >= 3)
                    || wrap_lines(
                        &format!("{} {}", joined(&cur), w.text),
                        lim.max_chars_per_line,
                    )
                    .len()
                        > lim.max_lines
            }
            _ => false,
        };
        if breaks {
            close(&mut caps, std::mem::take(&mut cur));
        }
        cur.push(w.clone());
    }
    close(&mut caps, cur);
    caps
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
