use super::{group_words, wrap_lines, GroupLimits};
use crate::edit::captions::CaptionWord;

fn w(start_ms: u32, end_ms: u32, text: &str) -> CaptionWord {
    CaptionWord {
        start_ms,
        end_ms,
        text: text.into(),
    }
}

#[test]
fn wrap_breaks_greedily_at_spaces_and_never_inside_a_word() {
    assert_eq!(
        wrap_lines("the quick brown fox jumps over the lazy dog", 20),
        vec!["the quick brown fox", "jumps over the lazy", "dog"]
    );
    assert_eq!(
        wrap_lines("supercalifragilistic", 10),
        vec!["supercalifragilistic"],
        "a word longer than the limit gets its own line rather than being split"
    );
    assert!(wrap_lines("", 42).is_empty());
    assert_eq!(
        wrap_lines("exactly ten", 11),
        vec!["exactly ten"],
        "a line at the limit fits"
    );
}

#[test]
fn a_caption_closes_when_the_next_word_would_push_it_past_four_and_a_half_seconds() {
    let lim = GroupLimits::default();
    let words: Vec<CaptionWord> = (0..6).map(|i| w(i * 1000, (i + 1) * 1000, "aaa")).collect();
    let caps = group_words(&words, &lim);
    assert_eq!(caps.len(), 2);
    assert_eq!((caps[0].start_ms, caps[0].end_ms), (0, 4000));
    assert_eq!(caps[0].text, "aaa aaa aaa aaa");
    assert_eq!((caps[1].start_ms, caps[1].end_ms), (4000, 6000));
    assert_eq!(caps[1].text, "aaa aaa");
}

#[test]
fn a_caption_closes_when_the_next_word_would_need_a_third_line() {
    let lim = GroupLimits::default();
    let long = "abcdefghijklmnopqrst";
    let words: Vec<CaptionWord> = (0..5).map(|i| w(i * 100, i * 100 + 90, long)).collect();
    let caps = group_words(&words, &lim);
    assert_eq!(caps.len(), 2);
    assert_eq!(caps[0].words.len(), 4);
    assert_eq!(caps[1].words.len(), 1);
    assert_eq!(wrap_lines(&caps[0].text, lim.max_chars_per_line).len(), 2);
}

#[test]
fn a_long_silence_starts_a_new_caption() {
    let lim = GroupLimits::default();
    let caps = group_words(&[w(0, 400, "one"), w(1400, 1800, "two")], &lim);
    assert_eq!(caps.len(), 2, "a gap over max_gap_ms breaks the line");
    let tight = group_words(&[w(0, 400, "one"), w(900, 1300, "two")], &lim);
    assert_eq!(tight.len(), 1);
}

#[test]
fn a_sentence_end_closes_a_caption_that_already_has_something_in_it() {
    let lim = GroupLimits::default();
    let caps = group_words(
        &[
            w(0, 200, "Hello"),
            w(200, 400, "there"),
            w(400, 600, "world."),
            w(600, 800, "Next"),
            w(800, 1000, "one"),
        ],
        &lim,
    );
    assert_eq!(caps.len(), 2);
    assert_eq!(caps[0].text, "Hello there world.");
    assert_eq!(caps[1].text, "Next one");
    let short = group_words(
        &[w(0, 100, "Dr."), w(100, 300, "Smith"), w(300, 500, "spoke")],
        &lim,
    );
    assert_eq!(short.len(), 1);
}

#[test]
fn ids_are_sequential_and_every_caption_carries_its_own_words() {
    let lim = GroupLimits::default();
    let words: Vec<CaptionWord> = (0..6).map(|i| w(i * 1000, (i + 1) * 1000, "aaa")).collect();
    let caps = group_words(&words, &lim);
    assert_eq!(
        caps.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        vec!["c0", "c1"]
    );
    assert_eq!(caps[0].words.len(), 4);
    assert_eq!(caps[0].words[0], words[0]);
    for c in &caps {
        assert_eq!(c.start_ms, c.words.first().unwrap().start_ms);
        assert_eq!(c.end_ms, c.words.last().unwrap().end_ms);
        assert_eq!(
            c.text,
            c.words
                .iter()
                .map(|x| x.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}

#[test]
fn no_words_is_no_captions() {
    assert!(group_words(&[], &GroupLimits::default()).is_empty());
}

#[test]
fn the_default_limits_are_the_ones_the_spec_names() {
    let l = GroupLimits::default();
    assert_eq!(l.max_chars_per_line, 42);
    assert_eq!(l.max_lines, 2);
    assert_eq!(l.max_ms, 4500);
    assert_eq!(l.max_gap_ms, 700);
}
