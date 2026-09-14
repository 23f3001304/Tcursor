# src-tauri/src/export/cursor/path.rs

The cursor's OFFLINE path model (2026-09-14). The recorded route is split into RESTS - where the hand held still, which is also where every click happens - and the MOVES between them. Polish (the Cursor panel's Smoothness and Path Idealization) is applied to the moves only, with both ends pinned, so the drawn cursor is exactly where the real one was whenever it rested or clicked; only the glide between those points is idealized. Nothing here lags: a move arrives when the recording arrived.

*Why a model of the whole recording and not a filter:* the old `Cursor` was a causal exponential low-pass (always BEHIND the real cursor, so a click landed before the drawn cursor had arrived on the button) blended toward straight strokes between CLICKS (a rest without a click - hovering, pointing, reading - was not an anchor, so the idealized cursor spent whole seconds nowhere near where the hand actually was). The owner's ruling: smoothing and idealization must never change where the cursor WAS. A recording is not live, so the path can be shaped between the points that matter instead of filtered toward them.

## REST_MS

```rust
pub const REST_MS: u32 = 120;
```

How long the hand must hold still (within `rest_px`) for a pause to count as a rest. Measured to the first sample that LEFT the spot, so a silent gap - a mouse at rest emits no events - is a rest too. Shorter hesitations mid-move are part of the move (and are what Smoothness irons out).

## rest_px

```rust
pub fn rest_px(screen_w: u32) -> f32
```

The radius, in frame px, a resting hand may wander within: `screen_w / 480`, floored at 3 - about 4 px on a 1920-wide capture, 8 px on 4K, so hand tremor at any capture size reads as the same rest.

## PathModel

```rust
pub struct PathModel {
    pts: Vec<Pt>,
    arc: Vec<f32>,
    segs: Vec<Seg>,
    strokes: Vec<Option<Vec<(f32, f32)>>>,
    key: (f32, f32),
}
```

- `pts` - every event as a frame-local `(t, x, y)` sample (`to_frame` applied; out-of-order events dropped, the log is chronological).
- `arc` - cumulative raw arc length per sample, in px, so a move's progress and resampling are O(log n) lookups.
- `segs` - the route as alternating rests and moves (`segment`), each over a sample range `i0..=i1` and timed `[t0, t1]`: a rest holds from its first sample until the next move starts; a move runs from one rest's last sample to the next rest's first.
- `strokes` - per segment, a move's polished polyline (`build_stroke`), built lazily on first use and dropped whenever `key` changes.
- `key` - the `(smooth, ideal)` the cached strokes were built for.

### Used by

- `src-tauri/src/export/cursor/mod.rs` - `Cursor` owns one, built once from the event log, and asks it per frame whenever any polish is on.

## PathModel::new

```rust
pub fn new(events: &[MouseEvent], screen: &ScreenInfo) -> Self
```

Builds the model: samples, arc lengths, and the rest/move segmentation. Cost is one pass over the log plus the rest scan (`segment`); strokes are not built here.

## PathModel::at

```rust
pub fn at(&mut self, t_ms: u32, smooth: f32, ideal: f32) -> Option<(f32, f32)>
```

The polished position at `t_ms` in frame px, or `None` with no samples at all (the caller falls back to frame centre).

### Inputs

- `t_ms` - event time. Any order: the lookup is a binary search over `segs`, not a forward index, so a preview rewind needs no reset.
- `smooth` - `CursorSettings::smoothness`, 0..1: how glassy the glide is. Shapes a move's velocity profile (raw progress eased toward a smoothstep of the move's own duration) and de-jitters its route (the spatial window in `build_stroke`).
- `ideal` - `CursorSettings::path_idealize`, 0..1: how straight the route is. Pulls a move's polished polyline toward its chord.

### Implementation

1. A new `(smooth, ideal)` clears the stroke cache.
2. Find the segment: the last one whose `t0 <= t_ms`. Before the first segment -> `None` (the caller shows centre before the first event, as it always has).
3. A **rest** -> `rest_at`: raw linear interpolation inside the rest's own samples, holding the LAST sample once reached - a hand that emits no events has not moved, so the silent gap is the rest, not a glide toward the next sample.
4. A **move** -> `u = (t - t0) / (t1 - t0)` and `raw = progress(seg, t)`, the raw path's arc-length fraction at `t` (how far the hand had really travelled). `f = raw + (smoothstep(u) - raw) * smooth` - at 0 the recording's own timing, hesitations included; at 1 one clean eased stroke over the same span. The stroke (cached, `build_stroke`) is evaluated at `f`, with `f >= 1` returning its end vertex verbatim rather than a lerp an ulp off.

### Behaviors worth knowing (`path_tests.rs`)

- `rests_and_clicks_are_the_recording_verbatim_at_any_polish` - at every `(smooth, ideal)` combination: both clicks are their recorded positions exactly, the arrivals at B and C are on time (`t=1000`/`t=2000` are exactly B/C), and inside a rest the cursor is the raw hand (within its 1 px tremor).
- `idealize_straightens_the_detour_and_smoothness_alone_does_not` - a move arcing ~187 px off its chord: `ideal 1` rides the chord (< 1 px), `ideal 0.5` is half way (within 5 px), `smooth 1` alone keeps the route (> 120 px off) - smoothness never straightens.
- `smoothness_irons_out_jitter_and_hesitation_but_pins_both_ends` - a 60 px zig-zag's summed second differences fall by more than half at `smooth 1`, with both endpoints still exact.
- `a_click_mid_flight_is_still_hit_exactly` - a Down with no rest around it is an anchor: hit exactly at every polish.
- `a_gap_with_no_events_is_a_rest_not_a_glide` - one sample, silence, then a move: held at the sample through the silence (and until `LEAD_MS` before the first sample that left), on the new position at that sample's time, held at the last sample forever after.
- `no_polish_replays_the_recordings_own_timing` - a 40 ms hesitation mid-move (too short for a rest) is kept at `smooth 0` and eased through at `smooth 1`.
- `an_empty_log_answers_none`.

## segment

```rust
fn segment(pts: &[Pt], clicks: &[bool], r: f32) -> Vec<Seg>
```

The rest scan. From each sample, extend a run while the samples stay within `r` px of the run's FIRST sample; the run is a rest when it lasts `REST_MS` (measured to the first sample that left, so a gap counts) OR holds a click (however short - a click mid-flight is still an exact anchor). Moves fill the space between rests: from the previous rest's last sample to this rest's first, starting `move_start` before the first sample that left. A rest's `t1` is patched to the following move's `t0` once known. A recording with no rest at all is one move.

## move_start

```rust
fn move_start(pts: &[Pt], pb: usize) -> u32
```

When a move leaves the rest whose last sample is `pb`: `LEAD_MS` (16 ms) before the first sample that left, or the whole gap if shorter. The first vertex of a move is timed here, so a departure from a long silence is a short lead-in rather than a jump.

## pinned_average

```rust
fn pinned_average(p: &[(f32, f32)], w: usize) -> Vec<(f32, f32)>
```

Moving average with half-window `min(w, i, n-1-i)`: full strength mid-stroke, exactly the input at both ends. Prefix sums keep it O(n). `build_stroke` runs it twice (a triangular kernel) with `w = smooth * SMOOTH_SPAN (0.15) * n`, on a polyline resampled uniformly by raw arc length (about one point per 3 px, 8..240 points), then lerps every point toward the chord by `ideal`. Both ends are the raw endpoints by construction.
