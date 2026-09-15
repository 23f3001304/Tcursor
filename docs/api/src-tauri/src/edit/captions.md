# src-tauri/src/edit/captions.rs

The caption track's data model (`CaptionWord`, `Caption`), split out on its own for M5 the way `EffectRegion`/`CameraMove` already were. `edit::model::EditDoc.captions` is a `Vec<Caption>`; nothing here builds one - that is the ASR pass, a later task.

## CaptionWord

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CaptionWord { pub start_ms: u32, pub end_ms: u32, pub text: String }
```

One spoken word with its own timing, produced by the ASR word-level timestamps.

- `start_ms` / `end_ms` - *the word's span, OUTPUT-clock ms (ADDED-1: the same one-clock contract every other region list in `EditDoc` uses - `DOC_VERSION` 2, 0 = the first video frame). ASR itself runs on a capture-clock WAV; the shift into output time happens before a `Caption` is ever constructed.*
- `text` - *the word as transcribed, no surrounding whitespace.*

### Used by

- `src-tauri/src/edit/captions.rs` - `Caption.words`
- `src/shared/edit.ts` - `CaptionWord` TS mirror

## Caption

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Caption {
    pub id: String,
    pub start_ms: u32,
    pub end_ms: u32,
    pub text: String,
    #[serde(default)] pub words: Vec<CaptionWord>,
}
```

One caption line on the timeline: a grouped run of words (or hand-typed/split text with no word timings). Grouped from ASR output by a pure pass (`asr::group`) into lines of at most 42 characters / 2 lines / 4.5s, also breaking on a long silence or a sentence end.

- `id` - *stable string key for future `EditOp`s to target a specific caption without relying on list position, matching every other region's `id` convention (`Zoom.id`, `Cut.id`, ...).*
- `start_ms` / `end_ms` - *the line's span, OUTPUT-clock ms.*
- `text` - *the full caption line, rendered as one unit by `export/fx/caption/captionlayout.rs` (a later task).*
- `words` - *word-level timings within the line, for the word-by-word highlight (`CaptionStyle.highlight`). `#[serde(default)]` so a caption typed by hand or produced by a split has an empty `words` and the highlight simply turns off for it - not an error, not a missing field.*

### Used by

- `src-tauri/src/edit/model.rs` - `EditDoc.captions: Vec<Caption>`
- `src/shared/edit.ts` - `Caption` TS mirror

### Behaviors

- `a_doc_written_before_captions_existed_loads_with_an_empty_track` - a v2 doc with no `captions` key parses and `captions` is empty; adding the field is not a schema bump.
- `a_caption_round_trips_through_json_with_its_words` - a `Caption` with two `CaptionWord`s survives a full `EditDoc` serialize/deserialize round trip.
- `a_caption_without_words_still_parses` - JSON with no `words` key deserializes to an empty `Vec`.
