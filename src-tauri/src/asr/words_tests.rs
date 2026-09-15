use super::{is_special, words_from_tokens, RawToken};

fn tok(text: &str, t0: i64, t1: i64) -> RawToken {
    RawToken {
        text: text.into(),
        t0_cs: t0,
        t1_cs: t1,
    }
}

#[test]
fn a_leading_space_starts_a_new_word_and_pieces_join_the_current_one() {
    let w = words_from_tokens(&[
        tok(" hel", 100, 110),
        tok("lo", 110, 125),
        tok(" wor", 130, 145),
        tok("ld", 145, 160),
    ]);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].text, "hello");
    assert_eq!(
        (w[0].start_ms, w[0].end_ms),
        (1000, 1250),
        "centiseconds become ms"
    );
    assert_eq!(w[1].text, "world");
    assert_eq!((w[1].start_ms, w[1].end_ms), (1300, 1600));
}

#[test]
fn special_tokens_never_become_words() {
    assert!(is_special("[_BEG_]"));
    assert!(is_special("<|endoftext|>"));
    assert!(is_special("[_TT_240]"));
    assert!(!is_special(" hello"));
    let w = words_from_tokens(&[
        tok("[_BEG_]", 0, 0),
        tok(" hi", 10, 20),
        tok("<|endoftext|>", 20, 20),
    ]);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].text, "hi");
}

#[test]
fn a_word_with_no_leading_space_at_the_very_start_still_becomes_a_word() {
    let w = words_from_tokens(&[tok("Hi", 5, 15), tok(" there", 20, 40)]);
    assert_eq!(
        w.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(),
        vec!["Hi", "there"]
    );
}

#[test]
fn a_token_whose_end_is_not_after_its_start_borrows_the_next_boundary() {
    let w = words_from_tokens(&[tok(" ok", 100, 100), tok(" then", 120, 140)]);
    assert_eq!(w[0].start_ms, 1000);
    assert!(
        w[0].end_ms > w[0].start_ms,
        "a zero-length word is never emitted: {:?}",
        w[0]
    );
}

#[test]
fn the_last_word_falls_back_to_a_millisecond_when_it_has_no_successor_to_borrow_from() {
    let w = words_from_tokens(&[tok(" end", 200, 200)]);
    assert_eq!((w[0].start_ms, w[0].end_ms), (2000, 2001));
}

#[test]
fn whitespace_only_and_empty_tokens_are_dropped() {
    let w = words_from_tokens(&[tok(" ", 0, 10), tok("", 10, 20), tok(" real", 20, 30)]);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].text, "real");
}

#[test]
fn no_tokens_is_no_words() {
    assert!(words_from_tokens(&[]).is_empty());
}
