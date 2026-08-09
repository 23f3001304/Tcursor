# src-tauri/src/ai/backend/timeline.rs

Converts raw recording logs (mouse events, hotkey actions, cursor-type samples, typing timestamps) into a compact, human-readable plain-text transcript for the AI director. Pure and deterministic - identical inputs always produce identical output. The transcript is intentionally terse (one event per line, times in seconds) to minimize token usage; a 120-moment cap prevents context overflow for long recordings.

## serialize

```rust
pub fn serialize(
    log: &EventLog, actions: &[ActionEvent], cursor: &CursorTrack,
    typing: &[u32], dur_ms: u32, shift: i64,
) -> String
```

Builds the full transcript string from all recording data sources, on the OUTPUT clock.

### Inputs

- `log: &EventLog` - the mouse event log; provides `screen.w`, `screen.h`, and the list of `MouseEvent` items. *Why:* screen dimensions are needed for the 3x3 region classifier; `Down` events produce click lines that anchor the AI's zoom placement decisions.
- `actions: &[ActionEvent]` - hotkey action events. *Why:* `SetLayout` variants tell the model when the presenter switched layout mid-recording; hold-start/end events are not included since the AI produces its own zoom plan.
- `cursor: &CursorTrack` - sampled cursor-type data. *Why:* `IBeam` spans indicate the user was editing text, which the LLM uses to infer typing context independent of actual keystrokes.
- `typing: &[u32]` - typing timestamps in milliseconds. *Why:* keystroke bursts are merged into range lines (`start-ends typing`) that tell the LLM about active typing intervals without exposing key content.
- `dur_ms: u32` - total clip duration in milliseconds, already on the OUTPUT clock (the caller passes the true clip length). *Why:* the first line of the transcript identifies clip length so the LLM knows the full time range for zoom and trim placement. NOT shifted by `shift` - it is already correct.
- `shift: i64` - ms to ADD to every EVENT-clock timestamp in `log`/`actions`/`cursor`/`typing` to land on the OUTPUT clock, saturating at 0 (`edit::migrate::output_shift`'s recipe, reused rather than re-derived). *Why the transcript needs this at all:* the director's `AddZoomFull { at_ms }` ops are applied as OUTPUT-time zooms (`edit::ops::api::apply`), so a transcript built on the raw event clock would place every click ~`events_ms - video_start` (≈800 ms) away from the zoom the model meant to anchor to it.

### Returns

A `String` with one event per line, sorted ascending by timestamp, capped at 120 moments after the header. If the 120-moment cap is hit, a trailing `"(capped at 120 moments)"` line is appended. The first line is always `"clip <s>s, screen <w>x<h>"` with times formatted to one decimal place via the private `fmt` helper.

### Implementation

1. Collect all `EventKind::Down` mouse events from `log.events` into timestamped `"<t>s click (<x>,<y>) <region>"` moments, `t` shifted via the private `sh(t, shift)` helper (`(t as i64 + shift).max(0) as u32`). Region is one of nine labels (e.g. `"top-left"`, `"center"`) produced by the `pub(crate)` `region()` function, which divides the screen into equal horizontal and vertical thirds. *Why `pub(crate)` rather than private:* `ai::backend::narrate::label_for` reuses the same classifier to name the region a zoom's triggering click landed in, so a zoom's narration always agrees with what the transcript already showed the LLM.
2. Collect `ActionKind::SetLayout(id)` hotkey events as `"<t>s layout -> <id>"` moments (`t` shifted, variant name lowercased via `Debug + to_lowercase`). Other `ActionKind` variants are ignored.
3. Merge SHIFTED typing timestamps into contiguous bursts: consecutive timestamps within 1000 ms of each other are grouped into one span. Each span emits `"<start>-<end>s typing"`. A gap of >= 1000 ms between two (already-shifted) timestamps starts a new span. *Why merge:* reduces token count from per-keystroke lines to per-burst ranges.
4. Scan `cursor.samples` for runs of `CursorType::IBeam`, shifting each sample's `t` on read; collapse each run into a `"<start>-<end>s text field"` moment. The span end time is taken from the sample immediately before the first non-IBeam entry, or from the last sample if `IBeam` continues to the end.
5. Scan consecutive pairs of SHIFTED mouse event timestamps; any gap >= 2000 ms emits `"<start>-<end>s idle"`. *Why 2000 ms threshold:* shorter gaps are normal mouse movement; only substantial idle periods are signal for the AI. *Why shift before diffing rather than after:* a constant offset cancels out of the gap arithmetic either way, but shifting up front means every reported window boundary is already an output-clock timestamp, matching every other line.
6. Sort all collected `Moment` items by `t_ms`; truncate to 120 if over the cap; prepend the clip header (using the UNSHIFTED `dur_ms` - it's already output-clock); join with newlines and return.

### Behaviors

- `region_corners_and_center` - all nine region names resolve correctly for 1920x1080 coordinates at edges and center.
- `serialize_contains_expected_lines` - a synthetic log with `shift=0` produces expected click, layout, typing, and idle lines.
- `time_ordered` - all output lines (after the header) have non-decreasing start times.
- `deterministic` - calling `serialize` twice with identical inputs (including `shift`) produces identical output.
- `ibeam_span_emitted` - a `CursorTrack` with an `IBeam` entry followed by `Arrow` produces a `"text field"` line.
- `click_timestamps_shift_onto_the_output_clock` - a click at raw event t=3100 with `shift=-800` serializes as `"2.3s click"`, not `"3.1s click"`.
- `layout_typing_and_idle_timestamps_shift_identically` - with `shift=-800`, click/layout/typing/idle lines all move by the same amount; the `clip <dur>s` header line does not move.

### Used by

- `src-tauri/src/ai/commands.rs` - `build_plan` (the shared LLM pass behind `ai_autoedit` and `ai_plan`) calls `serialize` and passes the result as the user message to `ai::backend::ollama::chat`.
