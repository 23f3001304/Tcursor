# src-tauri/src/asr/group.rs

Words into caption lines. Pure, one left-to-right pass, no lookahead: a word either joins the caption in progress or opens the next one. Everything that decides which is a number in `GroupLimits`, so the whole rule set is readable in one struct and testable without an ASR run.

## GroupLimits

```rust
pub struct GroupLimits { pub max_chars_per_line: usize, pub max_lines: usize, pub max_ms: u32, pub max_gap_ms: u32 }
```

Defaults `42 / 2 / 4500 / 700`. The first three are the spec's (decision 3); `max_gap_ms` is plan ADDED-9's, alongside the sentence rule below.

*Why a silence and a sentence end were added to the spec's three:* a caption that runs straight through a two-second pause, or through a full stop, reads as one run-on thought even when it is inside every size limit. Both are places a reader already expects a new line, so breaking there costs nothing and reads better.

## wrap_lines

```rust
pub fn wrap_lines(text: &str, max: usize) -> Vec<String>
```

Greedy word wrap, measured in CHARACTERS (not bytes - `chars().count()`, so an accented word is not mis-measured). Never splits a word: one longer than `max` gets a line of its own and overflows, which the renderer shrinks to fit rather than hyphenating. Empty text is no lines at all, not one empty line.

Used twice: by `group_words` to decide whether one more word would need a third line, and by the renderer to lay the caption out - the same function on both sides, so what the grouping promised fits is what actually fits.

## group_words

```rust
pub fn group_words(words: &[CaptionWord], lim: &GroupLimits) -> Vec<Caption>
```

Words into caption lines, in order, with ids `c0`, `c1`, ... A word opens a new caption when ANY of these holds:

1. the caption would run past `max_ms` measured from its own first word's start;
2. the silence since the previous word's end exceeds `max_gap_ms`;
3. the previous word ended a sentence (`.`, `?` or `!`) AND the caption already holds three words or more;
4. the wrapped text including this word would need more than `max_lines` lines.

Each caption's `start_ms`/`end_ms` are its first and last word's, its `text` is its words joined by single spaces, and it carries those words, so the word-by-word highlight has what it needs and `Caption::text` and `Caption::words` can never disagree on what was said.

*Why the three-word floor on rule 3:* one or two words before a full stop is an abbreviation at least as often as a sentence. Without the floor, "Dr. Smith spoke" becomes two captions, one of them the single word "Dr.". Three is the smallest floor that fixes the common titles and initials without letting a genuinely short sentence run into the next one.

*Why no lookahead:* the alternative (balancing line lengths across a whole utterance) reads slightly better on paper and much worse in practice, because it needs the END of the utterance before it can emit the FIRST caption - which is the one thing captions cannot afford if this ever becomes a streaming pass.

### Behaviors

- `wrap_breaks_greedily_at_spaces_and_never_inside_a_word` - including a word longer than the limit, the empty string, and a line exactly at the limit.
- `a_caption_closes_when_the_next_word_would_push_it_past_four_and_a_half_seconds` - six one-second words become `[0, 4000]` "aaa aaa aaa aaa" and `[4000, 6000]` "aaa aaa".
- `a_caption_closes_when_the_next_word_would_need_a_third_line` - five 20-character words split 4 + 1, because two fit per 42-character line.
- `a_long_silence_starts_a_new_caption` - a 1000 ms gap breaks, a 500 ms gap does not.
- `a_sentence_end_closes_a_caption_that_already_has_something_in_it` - and the "Dr. Smith spoke" counter-case for the three-word floor.
- `ids_are_sequential_and_every_caption_carries_its_own_words` - ids, and the start/end/text/words agreement on every caption.
- `no_words_is_no_captions`, `the_default_limits_are_the_ones_the_spec_names`.
