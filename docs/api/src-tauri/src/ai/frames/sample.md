# src-tauri/src/ai/frames/sample.rs

WHEN the director looks. Pure, no I/O: given the recording's click times, its layout-switch times and the clip length, it decides which moments are worth a frame. The rule is that the recording's own events are what matter, with a coarse idle grid so a long stretch of nothing still gets described rather than being invisible to the model.

Every time here is on the OUTPUT clock (0 = the first video frame), the same clock the proxy plays on and every `EditOp` is stored on, so a sampled frame and the proposal it justifies always name the same instant.

## MAX_FRAMES

```rust
pub const MAX_FRAMES: usize = 16
```

The most frames one propose pass may send (binding decision 1). A local vision model pays for every image in both latency and context, and 16 is the point past which a 7B model on a laptop GPU stops finishing inside a usable wait.

## IDLE_EVERY_MS

```rust
pub const IDLE_EVERY_MS: u32 = 8_000
```

The idle grid's spacing: one frame every 8 seconds of clip time. Half of it is also the guard radius that stops a grid point from duplicating a real event's frame.

## FrameReason

```rust
pub enum FrameReason { Click, Layout, Idle }
```

Why this moment was chosen. Also its PRIORITY, ordered `Click < Layout < Idle`: when two candidates collide the more deliberate one wins, and when the cap bites the idle grid is what gets thinned first.

## FrameAt

```rust
pub struct FrameAt { pub t_ms: u32, pub reason: FrameReason }
```

One moment to sample: the output-clock time and why it was chosen. `Copy`, so `jpegs_at` can pair each one with its bytes without cloning.

## sample_times

```rust
pub fn sample_times(clicks: &[u32], layouts: &[u32], dur_ms: u32, cap: usize) -> Vec<FrameAt>
```

### Inputs

- `clicks` - output-clock times of every mouse-down in the recording (`ai::run::click_points`). Order does not matter.
- `layouts` - output-clock times of every layout switch (`ai::run::layout_times`).
- `dur_ms` - the clip's true length. A zero-length clip samples nothing.
- `cap` - the hard ceiling on the returned length, normally `MAX_FRAMES`.

### Returns

Moments sorted by `t_ms`, strictly ascending and never longer than `cap`. Never empty unless the clip is zero-length or every candidate sits past its end.

### Implementation

In this exact order:

1. Candidates: every `clicks[i]` as `Click`, every `layouts[i]` as `Layout`, and every multiple of `IDLE_EVERY_MS` in `[0, dur_ms)` as `Idle`.
2. Drop any candidate at or past `dur_ms`. *Why:* an event recorded after the last video frame has no frame to show for it.
3. Drop any `Idle` candidate within `IDLE_EVERY_MS / 2` of any `Click` or `Layout`. *Why:* a grid point next to a real event would spend one of 16 slots re-describing a moment already covered.
4. Sort by `(t_ms, reason)`, then walk greedily, skipping any candidate less than `MIN_GAP_MS` (400) after the last kept one. *Why:* a double-click is one moment; three frames of it would crowd out three other moments. The reason ordering is what makes a click beat a layout switch at the same instant.
5. If more than `cap` survive, `thin` keeps the real events first (`spread` over the `Click`/`Layout` list) and fills whatever slots remain with an evenly spread subset of the idle grid.
6. Re-sort by `t_ms` so the caller can label the images in play order.

### Behaviors

- A five-minute clip with 16 clicks in its first 16 seconds returns those 16 clicks and no idle frames at all. The idle grid is a floor for a quiet clip, never a competitor to what actually happened.
- `spread` uses `j * (len - 1) / (keep - 1)`, which pins both the first and the last element. The plainer `j * len / keep` drops the last event, and the last thing that happened in a recording is usually the point of it.

## spread

```rust
fn spread(src: &[FrameAt], keep: usize) -> Vec<FrameAt>
```

`keep` of `src`, evenly spaced by index, always including the first and the last. Returns `src` whole when `keep` is at least its length, empty when `keep` is 0, and just the first element when `keep` is 1 (there is no way to honour both ends with a single slot; the opening frame is the one that sets context).

## thin

```rust
fn thin(all: Vec<FrameAt>, cap: usize) -> Vec<FrameAt>
```

Splits the survivors into real events and idle grid, `spread`s the real events into at most `cap` slots, and spends whatever is left on the idle grid. The result is unsorted; `sample_times` re-sorts.

## rank

```rust
fn rank(r: FrameReason) -> u8
```

`Click` 0, `Layout` 1, `Idle` 2 - the tie-break that decides which of two candidates at the same instant survives the minimum-gap walk.
