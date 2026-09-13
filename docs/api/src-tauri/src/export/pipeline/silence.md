# src-tauri/src/export/pipeline/silence.rs

Remove silences (`docs/superpowers/specs/2026-09-13-time-remap-design.md`, section 7): ffmpeg's `silencedetect` over the recorded tracks, intersected when both exist (a stretch is silent only when mic AND system are), mapped onto the clip clock with the same shift the mux uses, padded so speech never starts mid-word, filtered, clamped into the trim. The editor applies the result as ONE `AddCuts`, so the whole batch is one undo step. No model, no network. The three thresholds are constants until someone asks for a settings row.

## THRESHOLD_DB

```rust
pub const THRESHOLD_DB: f32 = -35.0;
```

Anything quieter than this, for at least `MIN_MS`, is a silence (`silencedetect=n=-35dB`).

## MIN_MS

```rust
pub const MIN_MS: u32 = 700;
```

The shortest stretch worth cutting, measured AFTER padding (so an 700 ms silence needs 1000 ms of quiet to survive the two pads).

## PAD_MS

```rust
pub const PAD_MS: u32 = 150;
```

Kept on each side of a silence so a word's tail and the next one's attack survive the cut.

## parse_silencedetect

```rust
pub fn parse_silencedetect(stderr: &str) -> Vec<(f64, f64)>
```

`silence_start: x` / `silence_end: y | silence_duration: z` pairs out of ffmpeg's log, in seconds of the track's own clock. A start with no end (the file ends silent) runs to `f64::MAX`.

## to_clip_ms

```rust
pub fn to_clip_ms(spans_s: &[(f64, f64)], track_shift_ms: i64) -> Vec<(u32, u32)>
```

Track seconds to clip ms: `track_shift_ms` is `audio_shift_ms(track_ms, video_start, 0)`, the track's own start relative to the clip's first frame, exactly the alignment the mux applies. Clamped at 0; an open end stays `u32::MAX`.

## intersect

```rust
pub fn intersect(a: &[(u32, u32)], b: &[(u32, u32)]) -> Vec<(u32, u32)>
```

The stretches silent on BOTH tracks, sorted.

## pad_and_filter

```rust
pub fn pad_and_filter(spans: Vec<(u32, u32)>, pad_ms: u32, min_ms: u32, lo: u32, hi: u32) -> Vec<(u32, u32)>
```

Clamp into `[lo, hi]` (the resolved trim) first, then pad each side, then drop what is shorter than `min_ms`. An open end becomes the trim-out.

## detect

```rust
pub fn detect(paths: &ProjectPaths, meta: &RenderMeta) -> Result<Vec<(u32, u32)>>
```

Runs `ffmpeg -v info -i <track> -af silencedetect=n=-35dB:d=0.7 -f null -` on whichever of `mic.wav` and `system.wav` exist (reading stderr), intersects when both do, and returns padded, filtered, trim-clamped clip-time spans ready for `AddCuts`. The editor skips spans already inside a cut before applying.

## detect_silences

```rust
#[tauri::command]
pub async fn detect_silences(folder: String, app: tauri::AppHandle) -> Result<Vec<(u32, u32)>, String>
```

The editor's Remove silences button: `detect` on the warm preview session's own `RenderMeta`, off the main thread through `spawn_blocking` like every other `with_warm` command (`preview_track.md`).

### Behaviours

- `parses_pairs_and_an_unterminated_start_runs_to_the_end`.
- `seconds_become_clip_ms_through_the_track_shift_and_an_open_end_stays_open`.
- `a_stretch_is_silent_only_when_both_tracks_are`.
- `padding_shrinks_each_side_and_short_leftovers_are_dropped` - including an open end clamped to the trim-out.
