# src-tauri/src/asr/audio.rs

The recording's own audio, on the ASR's terms and then back on the doc's. Two halves that only look unrelated: picking and decoding a WAV to the 16 kHz mono f32 whisper.cpp takes, and the pure clock arithmetic that moves a word timed against that WAV onto the OUTPUT clock every region list in `EditDoc` uses (`DOC_VERSION` 2's one-clock contract).

**Why not `preview_synced.m4a` (plan ADDED-2):** that file is already aligned to the video start, so transcribing it would need no shift at all. But its alignment rides on ffmpeg `-itsoffset`/`-ss` container semantics - edit lists, encoder priming - that are worth trusting for playback and not worth trusting for word timing. The shift is re-derived arithmetically from `sync.json` instead, through the SAME `pipeline::audio_shift_ms` the export mux and the preview audio call, so there is never a second copy of the formula to drift.

## AsrSource

```rust
pub enum AsrSource { Mic, System }
```

Which of the recording's two tracks the transcription reads. A mic + system MIX is deliberately out of scope: mixing two tracks recorded on different clocks to feed a timestamp-sensitive decoder trades a small coverage gain for a whole new class of drift.

## pick_source

```rust
pub fn pick_source(paths: &ProjectPaths) -> Option<AsrSource>
```

`Mic` when `mic.wav` exists, else `System` when `system.wav` does, else `None` - the recording captured no audio and there is nothing to transcribe. Mic wins because it is the person talking, which is what a caption is for; a screen recording with only desktop audio still gets captions of whatever that audio says.

## wav_for

```rust
pub fn wav_for(paths: &ProjectPaths, src: AsrSource) -> PathBuf
```

The file behind an `AsrSource`.

## to_output_ms

```rust
pub fn to_output_ms(audio_ms: i64, shift_ms: i64) -> Option<u32>
```

Output-clock ms for an instant in the ASR wav, or `None` when it is still before the first video frame. `shift_ms` is `pipeline::audio_shift_ms(track_ms, video_start, 0)`: POSITIVE when the track started after the video, which is the usual case (the encoder is running before the mic opens).

## shift_words

```rust
pub fn shift_words(words: Vec<CaptionWord>, shift_ms: i64) -> Vec<CaptionWord>
```

A whole word list onto the output clock, with two deliberate edge rules:

- A word that ENDS before the first video frame is DROPPED. It did not happen as far as the export is concerned, and a caption pinned at 0 for speech nobody can see would be worse than silence.
- A word that STRADDLES the start is KEPT, with its start clamped to 0. Cutting a word in half reads worse than starting it a fraction early, and this is the common case when someone starts talking as they press record.

A word left with no positive length after the clamp is dropped too, so the caller never has to defend against one.

## source_shift_ms

```rust
pub fn source_shift_ms(paths: &ProjectPaths, src: AsrSource) -> i64
```

How far this recording's chosen track sits from the first video frame. Loads the event log, builds the same `sync.json`-backed `Timeline` the export does (`pipeline::timeline::build_timeline`, 60 fps fallback - the same call `edit::seed` and `edit::migrate` make), and hands `tl.mic_ms` / `tl.system_ms` and `tl.frames[0]` to `pipeline::audio_shift_ms`.

`0` when the event log or the timeline is missing, which is exactly what every other consumer of that function falls back to. An unshifted transcript of a recording with no sync data is still better than no transcript.

## decode_16k_mono

```rust
pub fn decode_16k_mono(wav: &Path) -> Result<Vec<f32>, String>
```

Decodes a WAV to the samples whisper.cpp takes, by spawning the bundled ffmpeg through `process::proc::ffcmd` with `-v error -i <wav> -f f32le -ac 1 -ar 16000 -` and reinterpreting stdout as little-endian `f32`. The same subprocess idiom every other media read in this tree uses, so there is no second audio decoder to keep in step with the first.

Errors rather than guessing on three conditions: ffmpeg exiting non-zero (its stderr is quoted into the message), empty output, and a byte count that is not a multiple of 4. That last one is the important one - a decode truncated mid-sample would shift every word after it by a fraction of a sample and produce captions that look almost right.

### Behaviors

- `an_audio_instant_lands_on_the_output_clock_by_adding_the_track_shift` - both signs of the shift.
- `a_word_spoken_before_the_video_started_is_dropped_and_a_straddling_one_is_clamped` - the two edge rules above, on one list.
- `to_output_ms_refuses_an_instant_that_is_still_negative`.
- `the_source_is_the_mic_when_there_is_one_and_the_system_track_otherwise` - including the no-audio `None`.
