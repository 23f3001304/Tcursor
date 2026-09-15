# src-tauri/src/asr/words.rs

Whisper tokens into words. whisper.cpp does not emit words: it emits sub-word PIECES, each with its own centisecond bounds, and marks a word boundary by a LEADING SPACE on the piece that opens one - `" hel" "lo" " wor" "ld"` for "hello world".

This pass is the pure, tested seam between the decoder and everything downstream, which is why `whisper::transcribe` returns raw tokens rather than words. It is also the seam a `whisper-cli` fallback (plan T0) would plug into with nothing else in the milestone changing.

## RawToken

```rust
pub struct RawToken { pub text: String, pub t0_cs: i64, pub t1_cs: i64 }
```

One token straight off the decoder: its text WITH whatever leading space it carried (that space is data, not formatting - dropping it before this pass would destroy the word boundaries), and its bounds in CENTISECONDS, the unit `whisper_token_data`'s `t0`/`t1` are in.

## is_special

```rust
pub fn is_special(t: &str) -> bool
```

Whisper's non-speech markers: `[_BEG_]`, `[_TT_240]`, `<|endoftext|>`, `<|1.00|>` and friends - `[_` or `<|` prefixed. They carry timing but no text, so they must never become a caption word. Checked by prefix rather than by an exhaustive list because the marker set differs between models and grows between whisper.cpp releases, while the two prefixes have not moved.

## words_from_tokens

```rust
pub fn words_from_tokens(tokens: &[RawToken]) -> Vec<CaptionWord>
```

Tokens into words. Specials and whitespace-only tokens are dropped first; of what is left, a token opens a NEW word when its raw text starts with a space, or when it is the first token kept (which is how a transcript that begins mid-utterance, with no leading space, still yields a word rather than nothing). Anything else appends its trimmed text to the word in progress and extends that word's end.

Every emitted word ends strictly after it starts. whisper.cpp sometimes emits `t1 == t0` on a piece, and a zero-length word is one no renderer can show and no word-by-word highlight can ever land on. Such a word borrows the NEXT word's start minus one millisecond - which keeps it inside its real span rather than inventing duration - and the last word in the list, having nothing to borrow from, falls back to a single millisecond.

Times are still on the ASR wav's own clock here. `audio::shift_words` is what moves them onto the output clock, deliberately as a separate step: this pass has no idea what a recording is.

### Behaviors

- `a_leading_space_starts_a_new_word_and_pieces_join_the_current_one` - the four-token "hello world" case, including the centisecond-to-ms conversion.
- `special_tokens_never_become_words` - both prefixes, and a full list with markers at each end.
- `a_word_with_no_leading_space_at_the_very_start_still_becomes_a_word`.
- `a_token_whose_end_is_not_after_its_start_borrows_the_next_boundary` and `the_last_word_falls_back_to_a_millisecond_when_it_has_no_successor_to_borrow_from` - the two halves of the zero-length rule.
- `whitespace_only_and_empty_tokens_are_dropped`, `no_tokens_is_no_words`.
