# src-tauri/src/session/record/segments.rs

The recorder's own ledger of mid-take source switches (2026-09-14, `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`): which extra mic and webcam segments a take produced and which display switches it made. Filled by the `switch_mic`, `mark_webcam_segment` and `switch_display` commands while the take runs, written into `sync.json` at Stop by `recorder_threads::save_session_files`. Shared as an `Arc<Mutex<_>>` so a command can push under its own short lock without holding the recorder's.

## SegmentLog

```rust
#[derive(Default, Debug)]
pub struct SegmentLog { pub mic: Vec<Segment>, pub webcam: Vec<Segment>, pub displays: Vec<DisplaySwitch> }
```

`mic` lists `mic_2.wav`, `mic_3.wav`... and never `mic.wav` (the first segment, which `mic_ms` places); `webcam` the same for `webcam_<n>.webm`.

## SegmentLog::set_display_size

```rust
pub fn set_display_size(&mut self, at_ms: u64, w: u32, h: u32)
```

Corrects the size of the display switch stamped `at_ms` (the last one with that stamp) to what its capture actually delivered - called through `gpu_frames::SizeHook` from the replacement capture's first frame, on the capture thread, under the log's own lock. `export::render::spans` crops the fitted picture by this size, and the window-rect estimate `switch_display` records first is wider than the capture by the invisible DWM borders. An unknown `at_ms` (the switch failed and its record was removed) changes nothing (`a_display_switch_takes_its_capture_s_first_frame_size`).

## SharedSegments

```rust
pub type SharedSegments = Arc<Mutex<SegmentLog>>;
```

## shared

```rust
pub fn shared() -> SharedSegments
```

A fresh, empty, shared log - one per take (`Running.segments`).

## recording_ms

```rust
pub fn recording_ms(clock: &dyn Clock, totals: &PauseTotals) -> u64
```

Now, on the recording clock: the capture clock with every paused span removed (`PauseTotals::stamp_ms`), the same clock `sync.json`'s frame times, `mic_ms` and every event stream are on, so a segment's `start_ms` lands where the merge expects it.

## next_name

```rust
pub fn next_name(stem: &str, ext: &str, extra_so_far: usize) -> String
```

The next segment's file name for a source: the first extra segment is `_2`, since the unnumbered file is the first segment (`next_name("mic", "wav", 0)` is `mic_2.wav`). Tests: `extra_segments_count_from_two`, `the_log_starts_empty_and_is_shared`.
