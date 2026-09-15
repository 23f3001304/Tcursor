# src-tauri/src/domain/time.rs

Defines the `Timestamp` newtype, the `Clock` trait for injectable time sources, and its wall-clock implementation `SystemClock`. The key architectural property is injectability - every component that stamps frames or audio start times accepts `Arc<dyn Clock>` so a test can hand in its own `Clock` (the record-path tests do) instead of real hardware or `sleep` calls. The old `FakeClock` here had no consumer outside its own test and was deleted in the cleanup of 2026-09-15.

## Timestamp

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);
```

A millisecond timestamp measured from the session-relative epoch established by the `SystemClock`'s construction time. Fully ordered; `pub u64` inner value for direct extraction.

- *Why a newtype rather than raw `u64`:* prevents accidental comparison between timestamps and other `u64` quantities (latency offsets, sample counts) without an explicit `.0` dereference. `PartialOrd + Ord` allow range checks and sorting without unwrapping.*

### Used by

- `capture/frame.rs` - `Frame.ts` carries the per-frame capture timestamp.
- `capture/frame_source.rs` - test frames are constructed with explicit `Timestamp` values.
- `audio/cpal_mic.rs` - `started` atom stores a `u64` from `capture_ms`; compared against 0 (= `Timestamp::ZERO.0`).
- `export/exporter.rs` - reads frame timestamps for A/V sync offset computation.

## Timestamp::ZERO

```rust
pub const ZERO: Timestamp = Timestamp(0);
```

Sentinel value representing the epoch or "not yet set". The mic start-time atomic is initialized to `0` and compared against this value to detect the first sample arriving on the callback thread.

## Timestamp::as_millis

```rust
pub fn as_millis(&self) -> u64
```

Returns the inner millisecond value. Equivalent to reading `.0` directly; provided for ergonomic method chaining.

### Returns

`u64` milliseconds since the session epoch.

## Timestamp::sub

```rust
impl std::ops::Sub for Timestamp {
    type Output = u64;
    fn sub(self, rhs: Timestamp) -> u64 { self.0 - rhs.0 }
}
```

Subtracts two `Timestamp`s and returns elapsed milliseconds as a plain `u64`. No underflow protection - in debug builds, `rhs > self` panics. Callers must ensure `self >= rhs`.

### Behaviors

- `timestamp_subtracts_to_elapsed_millis` - asserts `Timestamp(1750) - Timestamp(1000) == 750`.

## Clock

```rust
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}
```

Injectable time source. `Send + Sync` so it can be shared across threads via `Arc<dyn Clock>`. Returns a session-relative millisecond count. All code that stamps frames or audio start times depends on this trait.

- `now_ms() -> u64` - *Returns the current time in milliseconds relative to the clock's epoch. Monotonicity is expected by callers but not enforced by the trait contract.*

### Used by

- `audio/cpal_mic.rs` - `CpalMic::open` accepts `Arc<dyn Clock>`; used in the callback to stamp `started` with `capture_ms(clock.now_ms(), info)`.
- `capture/windows_capture.rs` - `WgcFrameSource::for_primary_display` accepts `Arc<dyn Clock>`; used in the WGC callback to stamp each `Frame.ts`.
- `session/recorder.rs` - constructs a `SystemClock` and distributes it to capture and mic.

## SystemClock

```rust
pub struct SystemClock { start: Instant }
```

Wall-clock implementation. The epoch is the `Instant` captured at construction.

- `start: Instant` - *Session epoch. All `now_ms()` calls measure elapsed time from this `Instant`, so timestamps are session-relative rather than absolute UTC, keeping values small and avoiding 64-bit overflow concerns.*

### Used by

- `session/recorder.rs` - `SystemClock::new()` at recording start.
- `audio/cpal_mic.rs` - `CpalMic::default_input` constructs one for the convenience path.

## SystemClock::new

```rust
pub fn new() -> Self
```

Records `Instant::now()` as the session epoch and returns a new `SystemClock`. Subsequent `now_ms()` calls return `start.elapsed().as_millis() as u64`. The cast truncates above ~585 million years elapsed, which is not a practical concern. Monotonic because `std::time::Instant` is guaranteed monotonic on all supported platforms.

### Returns

`Self` with `start` set to the current instant.
