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
pub struct Segment { pub clip_start: u32, pub clip_end: u32, pub factor: f64, pub out_start: f64, pub clip: usize }
```

One kept range of clip time `[clip_start, clip_end)` with the factor it plays at, the output time it begins at, and `clip`, the index of the clip it came from IN `doc.clips` AS WRITTEN - not a position in some filtered list - so `doc.clips[s.clip]` is always the clip that produced the segment. An inverted or empty clip is dropped and simply leaves a gap in the indices carried (with clips 0, 1, 2 and 1 dropped, the segments carry 0 and 2). 0 for every segment of a doc with no clips: the trim range is the clip list's index 0. `clipops::clamp_transition` and Batch 4's per-clip decoder both index `doc.clips` with this, which is why it is the written index. Segments are contiguous in output time and ordered in clip time WITHIN one clip; across a clip join the clip time can jump anywhere, including backwards. A gap between two segments of the same clip is a cut (or the trimmed-off outside).

## TimeMap

```rust
pub struct TimeMap { /* segments, total output duration, trim-in, plain flag */ }
```

Built once per doc; every reader below is pure and cheap enough to call per frame.

## TimeMap::build

```rust
pub fn build(trim: &Trim, cuts: &[Cut], speed: &[Speed], clips: &[Clip], full_dur_ms: u32) -> TimeMap
```

Normalisation, in order: `trim.resolve(full_dur_ms)` gives the kept range; cuts are clamped into it, sorted and merged when they overlap or touch; speed spans are clamped into it, sorted, a later span clamped to start at its predecessor's end, factors clamped to `[FACTOR_MIN, FACTOR_MAX]`, empty spans dropped; every kept piece is split at speed-span edges and tagged with the span's factor (1.0 outside). A cut inside a speed span removes those frames from the span. A cut covering the whole range leaves no segments.

**Clips.** That normalisation is the body of an outer loop over the clip ranges: an EMPTY clip list gives one range, `trim.resolve(full_dur_ms)` at index 0, which is exactly the paragraph above and reproduces every pinned table; a non-empty list gives one range per clip in OUTPUT order, `(src_in_ms, src_out_ms)` each clamped to `full_dur_ms`, an inverted or empty range dropped and the trim ignored. Cuts and speed spans stay in SOURCE time and are resolved inside each range, so a clip that shows a cut range simply has that piece missing and a clip that shows none of it is untouched; a range used twice yields its segments twice. `out_start` is declared before the loop and accumulates across the clips, so the output is the concatenation of each clip's kept pieces in clip order.

`clip_ranges` carries each range's index in `clips` alongside its bounds (`(usize, u32, u32)`), and `build` writes THAT into `Segment.clip` rather than the loop's own position: the filter that drops inverted and empty ranges would otherwise renumber every clip after a dropped one, and `clip_out_ms`, `clipops::clamp_transition` and Batch 4's decoder all index `doc.clips` directly. `plain` therefore compares the `(lo, hi)` parts only - it is true when the ranges are exactly `[trim.resolve(..)]` AND no range saw a cut or a span, so one clip narrower than the trim is not plain even with nothing else on the doc.

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

Output time of a clip time: the FIRST showing of that source instant. Two passes, because with clips the segments are no longer sorted in clip time. Pass one looks for the segment containing `clip_ms` and is piecewise linear inside it (`out_start + (t - clip_start) / factor`). Pass two runs when no segment shows it (it is inside a cut, outside the trim, or in a stretch no clip uses) and takes the segment with the SMALLEST `clip_start` greater than `clip_ms`, ties broken by the smaller `out_start`: the next source instant the viewer will see. With no such segment it is `out_dur_ms`. With one clip in source order that is exactly the old "next segment in order", which is why the trim/cuts/speed tables are unchanged; with the two-clip fixture, `out_of(4000)` is 0, because clip A (6000..9000) is where the next shown source instant lives.

## TimeMap::clip_of

```rust
pub fn clip_of(&self, out_ms: u32) -> u32
```

The inverse on the kept ranges (`clip_start + (o - out_start) * factor`). At or past the end it returns the last segment's `clip_end`; with no segments, the trim-in. `clip_of(out_of(t)) == t` on every kept `t`.

## TimeMap::crosses_boundary

```rust
pub fn crosses_boundary(&self, prev_out_ms: u32, out_ms: u32) -> bool
```

Whether the OUTPUT times `prev_out_ms` and `out_ms` are separated by a splice: the segments containing them (`seg_of_out`, the first segment whose output range has not ended) are `a` and `b` with `a < b`, and some join between them is non-contiguous (`segs[i].clip_end != segs[i + 1].clip_start`). That covers a cut and a clip join in one rule; a speed-span edge or a clip join that happens to be contiguous in source time is not a boundary, and neither is a backwards step or a pair inside one segment. A trim edge is never one either: `seg_of_out` is `None` outside the mapped range, so the trim's own edges are the ends of the output, not interior joins. Replaces `crosses_cut`, which asked the same question on the recording clock.

The predicate for a caller holding two OUTPUT times and no plan - Batch 4's preview loop. `FrameRenderer::walk_plan` no longer asks it: a walk along `frame_plan` holds plan indices, and `plan_boundaries` answers on those, without the rounding that lands this rule an entry late on a segment whose output length is fractional.

## TimeMap::clip_out_ms

```rust
pub fn clip_out_ms(&self, clip: usize) -> u32
```

The output length of one clip: the sum of `(clip_end - clip_start) / factor` over the segments carrying that index, rounded, and 0 for an index with no segments. `clip` is the index in `doc.clips` as written, so this takes the same index its caller holds; an index whose clip was DROPPED (inverted, empty, or wholly cut away) has no segments and yields 0, which is the honest answer - that clip contributes nothing to the output. `clipops` clamps a dissolve against it.

## TimeMap::frame_bounds

```rust
pub fn frame_bounds(&self, i: usize, fps: u64) -> Option<(u64, u64)>
```

The inclusive recording-frame index range of segment `i` at `fps`. The first segment floors its start (`k = clip_start * fps / 1000`) and the last floors its end, exactly today's trim semantics; an edge that meets a cut is exact: `ceil(start)` and `ceil(end) - 1`, so precisely the frames whose time lies inside the cut are removed. `None` when the range is empty at this frame rate.

## TimeMap::frame_plan

```rust
pub fn frame_plan(&self, fps: u64) -> Vec<u64>
```

For every output frame `j`, the recording frame it shows: within a segment with bounds `(k_start, k_end)` there are `floor((k_end - k_start) / factor) + 1` output frames, the `i`-th showing `k_start + floor(i * factor)`. A factor above 1 skips frames, one below 1 repeats them. The exporter walks this with `pipeline::plan_walk::PlanCursor`; `preview::render_frame` and `camera_track` walk the same plan so the preview's camera is the export's.

Monotone non-decreasing for a doc with no clips (or with clips in source order), which is what lets the exporter's decoders only ever advance. A REORDERED clip list breaks that: the plan is each clip's own plan concatenated, so it steps backwards at a clip join (the two-clip fixture ends its first clip at 89 and opens its second at 5). Batch 4's per-clip decoder is what consumes a plan like that; nothing seeds a reordered doc until then.

## TimeMap::plan_boundaries

```rust
pub fn plan_boundaries(&self, fps: u64) -> Vec<usize>
```

The ascending `frame_plan` indices at which a segment opens that is NOT contiguous in source time with the segment before it - the entries `FrameRenderer::walk_plan` snaps the cursor on, tested with `binary_search`. One walk over the segments using the same `frame_bounds` arithmetic and the same per-segment entry count as `frame_plan` (`floor((k_end - k_start) / factor) + 1`), so the running total is exactly the plan index of each segment's first entry.

`last_end` is the `clip_end` of the last segment that PRODUCED at least one entry, not of the last segment: one with no frames at this rate (`frame_bounds` is `None`) contributes nothing and does not touch it, so the join is judged between the segments that are actually in the plan. Index 0 is never pushed (`last_end` is `None` until a segment has produced entries), so `walk_plan` needs no `j > 0` guard.

Keyed on the PLAN rather than on the output clock, which is the whole difference from `crosses_boundary`. A segment whose output length is fractional puts its join between two whole output milliseconds - 0..1533 at 2x ends at 766.5 ms while `ms(23)` is 766 - so `crosses_boundary(ms(j - 1), ms(j))` fired at j = 24 where the segment's first entry is j = 23. `snap_cursor` resets the cursor smoother and its trail, so the entry it fires on IS the picture: one entry late is one visibly wrong frame.

A cut shorter than one frame still gets an index. A cut of 1003..1015 at 30 fps removes no plan entry at all (the plan is `0..=300`, exactly what it would be with no cut), yet the segments meet 1003 to 1015, so index 31 - the second segment's first entry - is a boundary and the cursor snaps there. That is intended: the splice is real even when no frame was dropped, and a smoother or a trail carried across it reads as the cursor sliding on its own.
