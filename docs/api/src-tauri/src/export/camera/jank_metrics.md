# src-tauri/src/export/camera/jank_metrics.rs

Jank metrics for the camera probe: first and second differences of a `Run` (`jank_scene.md`), expressed in the unit the viewer perceives (OUTPUT pixels of on-screen motion per frame), plus the spike ranking and the hypothesis classifier the probe's report tables are built from. Included under `#[cfg(test)]` from `camera/mod.rs`; `jank_probe_tests.rs`'s `mod filter` is where the fingerprint `0x58da_34b7_41e9_0fd3` over one of these runs is pinned, the number that must not move.

## Series

```rust
pub struct Series { pub name: &'static str, pub vel: Vec<f32>, pub acc: Vec<f32> }
```

One motion channel of a `Run` converted to screen pixels: `vel` per frame, `acc` its first difference (the jerk proxy), both index-aligned with the run (leading entries 0).

## series

```rust
pub fn series(r: &Run) -> Vec<Series>
```

The three channels. A pan of `dcx` source px shows as `scale * dcx` px of motion; a scale step `ds` slides content at the viewport edge by `FW / (2 * scale) * ds`.

## rms

```rust
pub fn rms(v: &[f32]) -> f32
```

Root mean square; 0 for an empty slice.

## max_abs

```rust
pub fn max_abs(v: &[f32]) -> (usize, f32)
```

Index and magnitude of the largest `|x|`.

## window

```rust
pub fn window(r: &Run, t0: u32, t1: u32) -> (usize, usize)
```

The index range `[a, b)` covering the ms window `[t0, t1]`, by binary search over `r.t`.

## at

```rust
pub fn at(r: &Run, v: &[f32], t: u32) -> f32
```

Linear interpolation of `v` at output time `t`, exactly what the editor's `camAt` does between `camera_track` samples, so a preview run can be compared to an export run at the SAME instant rather than at whatever timestamps each grid lands on.

## Spike

```rust
pub struct Spike { pub t: u32, pub i: usize, pub comp: &'static str, pub acc: f32, pub v0: f32, pub v1: f32 }
```

One ranked jerk event: when, which sample, which channel, the acceleration, and the velocities either side.

## top_spikes

```rust
pub fn top_spikes(r: &Run, n: usize) -> Vec<Spike>
```

The `n` largest `|acc|` events across all three channels, suppressing the ringing right after a spike (a same-channel sample within 2 steps of one already taken) so the table shows `n` distinct events rather than one event's decay.

## classify

```rust
pub fn classify(r: &Run, i: usize) -> String
```

Which hypothesis the spike at sample `i` belongs to, decided from the run's own recorded state and the region timetable rather than by eye: H2 a driver handoff or a region start, H1 ramp-in to hold, H5 hold to ramp-out, H6 the clamp engaging, releasing or riding the frame edge, else H4 cursor low-pass or raw sample jitter. "Near" a boundary means within 34 ms, two frames.

## print_spikes

```rust
pub fn print_spikes(r: &Run, n: usize)
```

The top-`n` table for one grid, one line per spike with its classification.

## print_jerk

```rust
pub fn print_jerk(tag: &str, r: &Run)
```

The whole-run jerk summary per channel: rms and max of `acc`, in screen px per frame squared.

### Used by

- `src-tauri/src/export/camera/jank_probe_tests.rs`, `jank_input_tests.rs` - the probe's report and the pinned fingerprint.
