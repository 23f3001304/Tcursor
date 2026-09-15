# src-tauri/src/export/remap.rs

The one clip-to-output clock map. Trim, cuts and speed spans become kept segments of **clip time** (0 = the first video frame, the clock every lane's pills sit on), each with a factor and the **output time** it starts at (the exported file's clock). Everything the viewer sees runs on the output clock: `edit::remap_doc` moves every region there before the renderer builds its tracks, so a 350 ms zoom-in is 350 ms of output whatever the speed span around it. Sources (video, cursor, clicks, webcam, audio, a video background) are sampled on the clip clock through `frame_plan`. Mirrored line for line by `src/shared/math/remap.ts` (`docs/api/src/shared/math/remap.md`) with the same f64 expression order, so both sides agree to the bit; the parity table in `remap_tests.rs` is the contract. Design: `docs/superpowers/specs/2026-09-13-time-remap-design.md`.

Naming: before this module, the renderer called the clip clock `out_t` because it was the export's clock too. `step_camera` now takes the two clocks separately (`t_clip_abs`, `t_out`), and the word "output" means only the remapped clock.

## FACTOR_MIN

```rust
pub const FACTOR_MIN: f64 = 0.25;
```

The slowest a speed span can run. Below 1.0 the exporter repeats frames (no interpolation).

## FACTOR_MAX

```rust
pub const FACTOR_MAX: f64 = 8.0;
```

The fastest a speed span can run. `atempo` chains in the audio mux cover the same range.

## Segment

```rust
pub struct Segment { pub clip_start: u32, pub clip_end: u32, pub factor: f64, pub out_start: f64 }
```

One kept range of clip time `[clip_start, clip_end)` with the factor it plays at and the output time it begins at. Segments are contiguous in output time and ordered in clip time; a gap between two of them is a cut (or the trimmed-off outside).

## TimeMap

```rust
pub struct TimeMap { /* segments, total output duration, trim-in, plain flag */ }
```

Built once per doc; every reader below is pure and cheap enough to call per frame.

## TimeMap::build

```rust
pub fn build(trim: &Trim, cuts: &[Cut], speed: &[Speed], full_dur_ms: u32) -> TimeMap
```

Normalisation, in order: `trim.resolve(full_dur_ms)` gives the kept range; cuts are clamped into it, sorted and merged when they overlap or touch; speed spans are clamped into it, sorted, a later span clamped to start at its predecessor's end, factors clamped to `[FACTOR_MIN, FACTOR_MAX]`, empty spans dropped; every kept piece is split at speed-span edges and tagged with the span's factor (1.0 outside). A cut inside a speed span removes those frames from the span. A cut covering the whole range leaves no segments.

## TimeMap::identity

```rust
pub fn identity(full_dur_ms: u32) -> TimeMap
```

No trim, no cuts, no speed spans: every clock function is the identity and the plan is every frame.

## TimeMap::is_plain

```rust
pub fn is_plain(&self) -> bool
```

True when there are no cuts and no speed spans (a trim may exist). A plain map's `frame_plan` is exactly `k_in ..= k_last` from `pipeline::trim_frame_bounds`, and every evaluator sees the clip clock, so the export and the mux are byte-identical to what they were before this module existed. Pinned by `a_trim_only_map_reproduces_trim_frame_bounds_exactly`.

## TimeMap::segments

```rust
pub fn segments(&self) -> &[Segment]
```

## TimeMap::out_dur_ms

```rust
pub fn out_dur_ms(&self) -> u32
```

The exported length: the sum of every segment's `(clip_end - clip_start) / factor`, rounded. Drives the transport readout and the export dialog's length; the mux keeps using the frame count for `-t`.

## TimeMap::out_of

```rust
pub fn out_of(&self, clip_ms: u32) -> u32
```

Output time of a clip time. Piecewise linear inside a segment (`out_start + (t - clip_start) / factor`); inside a gap it is the NEXT segment's start, which is the frame the viewer sees next; at or past the last segment it is `out_dur_ms`.

## TimeMap::clip_of

```rust
pub fn clip_of(&self, out_ms: u32) -> u32
```

The inverse on the kept ranges (`clip_start + (o - out_start) * factor`). At or past the end it returns the last segment's `clip_end`; with no segments, the trim-in. `clip_of(out_of(t)) == t` on every kept `t`.

## TimeMap::gap_containing

```rust
pub fn gap_containing(&self, clip_ms: u32) -> Option<(u32, u32)>
```

The gap a clip time falls in as `(previous kept end, next kept start)`; the trailing gap runs to `u32::MAX`. `None` on a kept range. The preview uses it to jump the media over a cut and to tell a cut jump from a seek.

## TimeMap::crosses_cut

```rust
pub fn crosses_cut(&self, prev_k: u64, k: u64, fps: u64) -> bool
```

Whether the recording frames skipped between two consecutive plan entries include a cut: the frame right after `prev_k` lies in a gap. A speed-span skip has no gap, so it is false. `FrameRenderer::walk_plan` snaps the cursor on it.

## TimeMap::frame_bounds

```rust
pub fn frame_bounds(&self, i: usize, fps: u64) -> Option<(u64, u64)>
```

The inclusive recording-frame index range of segment `i` at `fps`. The first segment floors its start (`k = clip_start * fps / 1000`) and the last floors its end, exactly today's trim semantics; an edge that meets a cut is exact: `ceil(start)` and `ceil(end) - 1`, so precisely the frames whose time lies inside the cut are removed. `None` when the range is empty at this frame rate.

## TimeMap::frame_plan

```rust
pub fn frame_plan(&self, fps: u64) -> Vec<u64>
```

For every output frame `j`, the recording frame it shows: within a segment with bounds `(k_start, k_end)` there are `floor((k_end - k_start) / factor) + 1` output frames, the `i`-th showing `k_start + floor(i * factor)`. A factor above 1 skips frames, one below 1 repeats them. Monotone non-decreasing, so the exporter's decoders only ever advance. The exporter walks this with `pipeline::plan_walk::PlanCursor`; `preview::render_frame` and `camera_track` walk the same plan so the preview's camera is the export's.
