# src-tauri/src/session/record/pause_clock.rs

The video paths' view of the exact-span paused-time ledger (`PauseTotals`), which is stamped at the real pause/resume instants under the recorder lock. Both capture paths ask it, once per arriving frame, where that frame sits on the ONE recording clock: the `sync.json` timestamp and the encoder PTS come out of a SINGLE ledger read, so `video.mp4`'s own timeline and `sync.json` - and therefore the audio WAVs, cursor, actions and zoom streams, which subtract the same ledger - cannot disagree about how long a pause was.

**What changed (sweep-2, findings C1 + H2).** This module used to own a `paused_ms` accumulator that inferred the paused span from the gap between consecutive PAUSED frame arrivals. WGC delivers frames on content change (`MinimumUpdateIntervalSettings` is a floor on the rate, not a heartbeat), so a pause over a static desktop produced at most one paused sample and removed 0% of the pause, while a pause over a busy desktop removed ~100% of it - the recording's reported duration was a function of what the screen happened to be doing, and that number sets `full_dur_ms`, `clip_ms` and `trim.out_ms`. It also only shifted the `sync.json` timestamps, never the video's own PTS.

## HNS_PER_MS

```rust
const HNS_PER_MS: i64 = 10_000;
```

100-nanosecond units per millisecond - the Media Foundation encoder's timestamp unit, and the only unit conversion in the module.

## FrameTick

```rust
pub struct FrameTick {
    pub sync_ms: u64,
    pub pts_100ns: i64,
}
```

Where one arriving frame sits on the recording clock. Both fields come from one `PauseClock::tick` call, i.e. one ledger read, on purpose: computing them from two separate reads would let a pause/resume land in between and put the two clocks back out of step.

- `sync_ms: u64` - capture time with every paused span removed; the value appended to `sync.json`'s `frames[]`.
- `pts_100ns: i64` - the same instant expressed as an encoder PTS, in 100ns units, zero at the first ENCODED frame (not at clock zero, so a take whose first frames arrive late - or during a leading pause - still starts at 0).

## PauseClock

```rust
pub struct PauseClock {
    totals: Arc<PauseTotals>,
    base_ms: Option<u64>,
    last_ms: Option<u64>,
}
```

- `totals: Arc<PauseTotals>` - the shared ledger, the same `Arc` every input tracker holds. *Why shared rather than a private accumulator:* it is stamped at the pause/resume toggles under the recorder lock, so its span is exact regardless of frame arrivals.
- `base_ms: Option<u64>` - `sync_ms` of the first encoded frame; the origin `pts_100ns` is measured from. `None` until that frame arrives.
- `last_ms: Option<u64>` - `sync_ms` of the previous encoded frame, used to refuse a non-advancing tick.

### Used by

- `src-tauri/src/session/record/gpu_frames.rs` - `Cap` holds one, calling `tick` once per `on_frame_arrived`.
- `src-tauri/src/session/record/recording_session.rs` - `RecordingSession` holds one, calling `tick` from `pump_once`.

## PauseClock::new

```rust
pub fn new(totals: Arc<PauseTotals>) -> Self
```

A clock over the given ledger, with no frames seen yet.

## PauseClock::tick

```rust
pub fn tick(&mut self, now_ms: u64, paused: bool) -> Option<FrameTick>
```

Place the frame that arrived at wall-clock `now_ms`.

### Inputs

- `now_ms: u64` - the wall-clock reading at frame arrival (from `Clock::now_ms()` in the GPU path, or the frame's own already-clock-derived `ts.0` in the legacy path). *Why not always a fresh clock read:* the legacy path's `Frame::ts` is already the wall-clock time the frame was captured, so reusing it avoids threading an extra `Clock` through `RecordingSession`.
- `paused: bool` - whether capture is currently paused for this tick.

### Returns

`None` means "drop this frame" - either capture is `paused`, or the pause-compressed time did not advance past the last encoded frame. The second case is reachable when a whole pause/resume lands between reading the clock and reading the pause flag; encoding it would hand the encoder a non-increasing PTS and put a duplicate timestamp in `sync.json`.

Otherwise `Some(FrameTick)` with the `sync.json` timestamp and the matching encoder PTS.

### Implementation

1. `paused` -> `None`.
2. `sync_ms = totals.stamp_ms(now_ms)` - one ledger read, serving both outputs.
3. If `sync_ms <= last_ms` -> `None`; otherwise record it as the new `last_ms`.
4. `base_ms` is initialised to this `sync_ms` on first use; `pts_100ns = (sync_ms - base) * HNS_PER_MS`.

### Behaviors

- `first_tick_is_unadjusted_and_starts_the_pts_at_zero`: an unpaused tick at 1000 with an untouched ledger gives `sync_ms 1000`, `pts_100ns 0`.
- `paused_ticks_are_dropped`: consecutive paused ticks all return `None`.
- `pause_with_no_frames_at_all_is_still_fully_removed`: ledger paused 1000..3000 with ZERO frames in between (the static-desktop case the old accumulator scored as 0 ms); a frame at 3100 lands at 1100.
- `accumulates_across_multiple_pause_episodes`: two episodes (200 ms then 150 ms) both count; a frame at 700 lands at 350.
- `tick_during_an_open_pause_is_dropped`: a mid-pause tick returns `None`, so the encoder never sees the paused span.
- `encoder_pts_is_exactly_the_sync_timestamp_rebased`: across a 30 s pause no frame observed, `pts_100ns == (sync_ms - first sync_ms) * 10_000` for every encoded frame. This is the C1 seam.
- `pts_is_relative_to_the_first_encoded_frame`: a pause that ends before any frame arrives still produces `pts_100ns == 0` for the first frame.
- `non_advancing_tick_is_dropped`: a short pause/resume that compresses a later arrival back onto an earlier instant yields `None` rather than a duplicate timestamp.
