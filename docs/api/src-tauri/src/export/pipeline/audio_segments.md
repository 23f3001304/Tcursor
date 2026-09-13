# src-tauri/src/export/pipeline/audio_segments.rs

The audio half of the time remap (`docs/superpowers/specs/2026-09-13-time-remap-design.md`, section 4): the kept segments as one ffmpeg filter chain that trims each out of the mixed track, re-times a speed span with `atempo` (pitch preserved) and concatenates them in order. Pure string building, so `audio_segments_tests.rs` pins the exact commands; `audio_mux::mux_args` inserts the chain after its volume/mix step. The identity (one segment at 1x, or none) emits no chain, which keeps the no-cut mux command byte-identical to the pre-remap one.

## AudioSeg

```rust
pub struct AudioSeg { pub start_s: f64, pub end_s: f64, pub factor: f64 }
```

One kept range in seconds of TRIMMED-video time (0 = the exported file's first frame; the mux's per-input offsets already put every track there), with the factor it plays at.

## atempo_chain

```rust
pub fn atempo_chain(factor: f64) -> String
```

`atempo` accepts 0.5..2 per instance on every ffmpeg since 4.x (the bundled 8.1 takes more, but the chain is the portable form), so larger factors are chained: 4 is `atempo=2,atempo=2`, 3 is `atempo=2,atempo=1.5`, 0.25 is `atempo=0.5,atempo=0.5`. Empty for 1.0.

## segment_chain

```rust
pub fn segment_chain(input: &str, segs: &[AudioSeg], output: &str) -> Option<String>
```

The chain from `input` (a labelled stream, `[x]`) to `output` (`[a]`): `asplit` into one branch per segment, each `atrim=start:end,asetpts=PTS-STARTPTS[,atempo...]`, then `concat=n=N:v=0:a=1`. A single re-timed segment skips the split and the concat. `None` for the identity, so the caller keeps its pre-remap command untouched.

## audio_segs

```rust
pub fn audio_segs(map: &TimeMap, out_fps: u64, trim_in_q_ms: u64) -> Vec<AudioSeg>
```

The map's segments in trimmed-video seconds: `trim_in_q_ms` is the frame-floored clip time of output frame 0 (`plan[0] * 1000 / fps`), the same shift the mux applies to each input through `audio_shift_ms`. A segment with no frame at this rate contributes no audio either.

### Behaviours

- `atempo_chains_stay_inside_ffmpegs_classic_range`.
- `the_identity_emits_no_chain_so_the_mux_command_is_unchanged`.
- `one_retimed_segment_needs_no_split_or_concat`.
- `segments_split_trim_setpts_tempo_and_concat_in_order` - the exact two-segment chain.
- `audio_segs_come_from_the_map_in_trimmed_video_seconds` - on the parity fixture with `trim_in_q = 500`.
