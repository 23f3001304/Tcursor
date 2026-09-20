# src-tauri/src/export/pipeline/audio_segments.rs

The audio half of the time remap (`docs/superpowers/specs/2026-09-13-time-remap-design.md`, section 4): the kept segments as one ffmpeg filter chain that trims each out of the mixed track, re-times a speed span with `atempo` (pitch preserved) and concatenates them in order. Pure string building, so `audio_segments_tests.rs` pins the exact commands; `audio_mux::mux_args` inserts the chain after its volume/mix step. The identity (one segment at 1x, or none) emits no chain, which keeps the no-cut mux command byte-identical to the pre-remap one.

## AudioSeg

```rust
pub struct AudioSeg { pub start_s: f64, pub end_s: f64, pub factor: f64 }
```

One kept range in seconds of TRIMMED-video time (0 = the exported file's first frame; the mux's per-input offsets already put every track there), with the factor it plays at.

## RAMP_S

```rust
pub const RAMP_S: f64 = 0.020;
```

20 ms: the length of the linear `afade` `segment_chain` puts at the start and end of every branch in a multi-segment chain, short enough to be inaudible as a fade and long enough to kill the click a hard `atrim` join leaves at an arbitrary sample boundary.

## atempo_chain

```rust
pub fn atempo_chain(factor: f64) -> String
```

`atempo` accepts 0.5..2 per instance on every ffmpeg since 4.x (the bundled 8.1 takes more, but the chain is the portable form), so larger factors are chained: 4 is `atempo=2,atempo=2`, 3 is `atempo=2,atempo=1.5`, 0.25 is `atempo=0.5,atempo=0.5`. Empty for 1.0.

## segment_chain

```rust
pub fn segment_chain(input: &str, segs: &[AudioSeg], output: &str) -> Option<String>
```

The chain from `input` (a labelled stream, `[x]`) to `output` (`[a]`): `asplit` into one branch per segment, each `atrim=start:end,asetpts=PTS-STARTPTS[,atempo...][,afade...]`, then `concat=n=N:v=0:a=1`. Every branch of this multi-branch form opens and closes on a `RAMP_S` (20 ms) linear `afade` - in at the branch's own start, out at its own end - because a clip join and a cut join are the same discontinuity in the same `concat` and click the same way: one rule on every branch is simpler than telling the two kinds of join apart, and this function cannot tell them apart anyway without the map that built the segments. A branch shorter than two ramps (under 40 ms of OUTPUT audio, i.e. `(end_s - start_s) / factor < 2 * RAMP_S`) halves them instead, so the in-fade and the out-fade meet at the branch's midpoint rather than overlapping past it, and a branch with no OUTPUT length at all halves away to nothing: `d = RAMP_S.min(out / 2.0)` is exactly 0 when `out` is 0, and `ramp` then emits no `afade` clause rather than one of zero duration. A single re-timed segment skips the split, the concat AND the ramp - it has no join to declick, so ramping it would only shave real audio off both ends of an otherwise-untouched clip - and `None` for the identity does the same for a plain project; both keep their pre-remap, pre-ramp command exactly as it was, so a project with no cuts, no speed spans and no clips produces the mux command it always produced.

## audio_segs

```rust
pub fn audio_segs(map: &TimeMap, out_fps: u64, trim_in_q_ms: u64) -> Vec<AudioSeg>
```

The map's segments in trimmed-video seconds, in the map's own OUTPUT order (`map.segments()`'s order - the clip list's order, not source order) - the order `segment_chain`'s `concat` needs them in to play back correctly once clips reorder. `trim_in_q_ms` is the shift every segment is measured from (the caller passes `audio_origin_q`'s result), the same shift `audio_shift_ms` applies to each mux input. `start_s` is clamped at zero: a segment can start up to one output frame before `trim_in_q_ms` when its OWN first frame ceils to a later recording frame than the plan's overall minimum (`frame_bounds` ceils every non-first segment's start), and without the clamp that residual would ask ffmpeg's `atrim` for a small negative start. A segment with no frame at this rate contributes no audio either.

## audio_origin_q

```rust
pub fn audio_origin_q(plan: &[u64], out_fps: u64) -> u64
```

The frame-floored clip time that output frame 0 is measured from, for both `audio_segs`' `trim_in_q_ms` and the mux's own per-track shift: `min(plan) * 1000 / out_fps`, not `plan[0] * 1000 / out_fps`. Every branch's `atrim=start=` and every track's shift are built from this one value, so it has to be the EARLIEST source frame the export uses, not merely the plan's first entry. On a monotone plan (no clips, or clips kept in source order) the minimum IS the first entry, so this reproduces `plan[0] * 1000 / out_fps` exactly and nothing moves for an existing project. After a reorder, the clip that plays FIRST can open on a LATER source frame than a clip that plays after it; measuring every branch from `plan[0]` there would ask a later branch's `atrim=start=` for a negative number, which is exactly the bug this function exists to close. An empty plan yields 0.

### Behaviours

- `atempo_chains_stay_inside_ffmpegs_classic_range`.
- `the_identity_emits_no_chain_so_the_mux_command_is_unchanged`.
- `one_retimed_segment_needs_no_split_or_concat`.
- `audio_segs_come_from_the_map_in_trimmed_video_seconds` - on the parity fixture with `trim_in_q = 500`.
- `the_origin_is_the_earliest_source_frame_the_plan_uses` - the parity fixture's monotone plan reproduces `plan[0]` (500 ms); the clips fixture's reordered plan opens on frame 60 but its minimum frame is still 5, so its origin is still 500 ms, the same as the unsplit fixture's.
- `a_reordered_map_trims_audio_in_output_order_and_never_asks_for_a_negative_start` - the clips fixture's six segments through `audio_origin_q`, every `start_s` non-negative and in the clip list's own order.
- `a_clip_whose_first_frame_ceils_above_the_origin_is_clamped_rather_than_negative` - a two-clip map whose second clip's 505 ms start ceils to frame 6 (600 ms), the plan's own minimum, so that segment's unclamped start would be 505 - 600 = -95 ms; clamped to 0.
- `every_branch_of_a_multi_segment_chain_ramps_in_and_out` - the exact two-segment chain with its `afade` pair on each branch; replaces the old `segments_split_trim_setpts_tempo_and_concat_in_order`, which pinned the same two segments before ramps existed.
- `a_branch_shorter_than_two_ramps_halves_them_rather_than_overlapping` - a 20 ms branch gets a 10 ms in-fade and a 10 ms out-fade instead of two overlapping 20 ms ones.
