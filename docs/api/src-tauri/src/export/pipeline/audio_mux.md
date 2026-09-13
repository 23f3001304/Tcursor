# src-tauri/src/export/pipeline/audio_mux.rs

Muxes the encoded silent video with recorded microphone and/or system audio into `final.<ext>` (`<ext>` from the export's `Format`). Handles all four audio combinations (both tracks, one track, no tracks), applies per-track A/V sync offsets, applies per-track volume gain before muxing, caps muxed audio to the (trimmed) video's own duration, and picks the right audio codec for the container.

## mux

```rust
pub fn mux(tmp: &Path, paths: &ProjectPaths, format: Format, mic_shift_ms: i64, sys_shift_ms: i64, mic_vol: f32, sys_vol: f32, out_dur_ms: u64, segs: &[AudioSeg]) -> Result<()>
```

**Time remap.** `segs` (`audio_segments::audio_segs`) is the kept ranges in trimmed-video seconds. `mux_args` appends `audio_segments::segment_chain` after the volume/mix step: the mix (or the lone track) lands on `[x]` and the chain trims, re-times (`atempo`, pitch preserved) and concatenates it into `[a]`. The identity (one segment at 1x, or none) adds nothing, so a no-cut export runs the exact pre-remap command, pinned by `without_segments_the_two_track_and_one_track_commands_are_the_pre_remap_ones`; `a_cut_and_a_double_speed_span_shorten_the_muxed_audio_to_match` runs the real ffmpeg on a lavfi fixture and checks the length.

Combines the temporary video file with whichever audio tracks exist and writes `<project_folder>/final.<ext>`.

### Inputs

- `tmp: &Path` - path to the composited silent video (written by the exporter's encoding step). *Why:* the video is encoded first, then audio is added in a single pass so the video stream is copied without re-encoding (`-c:v copy`).*
- `paths: &ProjectPaths` - project path resolver providing `paths.mic()`, `paths.system()`, and `paths.folder`. *Why:* audio track paths are project-specific and must be resolved through `ProjectPaths`, not hard-coded.*
- `format: Format` (`export::settings::Format`) - the export's container/codec choice. *Why:* determines the final filename's extension (`format.extension()`), whether audio is even attempted (`format.supports_audio()` - `false` for `Gif`), and which audio encoder to use (`format.audio_codec()` - `"aac"` for `Mp4`, `"libopus"` for `WebM`).*
- `mic_shift_ms: i64` - signed A/V offset for the microphone track in milliseconds. Positive = delay mic (mic started early relative to video); negative = trim mic lead. *Why:* microphone capture may start before or after video capture; this offset corrects drift measured from `sync.json`.*
- `sys_shift_ms: i64` - signed A/V offset for the system audio track. *Why:* system audio comes from a separate capture device and may have independent drift.*
- `mic_vol: f32` - linear gain multiplier applied to the mic track (`Settings.audio_mic_volume`; 0 = muted, 1 = unchanged, up to 1.5). *Why linear rather than dB:* matches ffmpeg's `volume` filter's default unit and the 0..150% slider in `AudioPanel`, so no conversion is needed at either end.*
- `sys_vol: f32` - linear gain multiplier applied to the system-audio track (`Settings.audio_sys_volume`). Same range as `mic_vol`.*
- `out_dur_ms: u64` - the exact duration of the encoded (trimmed) video in milliseconds, `(total_out * 1000) / out_fps` from the exporter's own trimmed frame count. Applied as `-t` on the audio paths only. *Why:* recorded audio files run the full capture length regardless of trim, and only trim-IN was ever applied (via `add_offset`'s `-ss`); without a `-t` cap, a 60s recording trimmed to 10s of video would mux in the full ~60s of audio, leaving the player holding on the last video frame while audio keeps playing. Ignored on the no-audio rename path (nothing to cap).*

### Returns

`Result<()>` - `Ok` on success. `Err` if ffmpeg exits non-zero or the no-audio rename fails.

### Implementation

1. Compute `final_path = paths.folder / "final.<ext>"` (`<ext>` from `format.extension()`).
2. Check `mic.exists()` and `system.exists()`, both additionally gated on `format.supports_audio()` - so `Gif` always behaves as if neither track exists, regardless of what was actually recorded.
3. **Both tracks:** build an `AudioTrack` for mic and system, and call `run_ffmpeg` with `mux_args`' output (three inputs: video, mic, system).
4. **One track:** build a single `AudioTrack` for whichever exists, and call `run_ffmpeg` with `mux_args`' output (two inputs: video + that track).
5. **No audio** (nothing recorded, or `format` can't carry it at all - `Gif`): remove any existing `final.<ext>`, rename `tmp` to `final.<ext>` via `fs::rename`. *Why rename not copy:* avoids duplicating a potentially large video file when there is nothing to mux. No volume filter or duration cap applies (nothing to mux).*
6. On audio paths, delete `tmp` after a successful ffmpeg mux (superseded by `final.<ext>`).

### Behaviors worth knowing

- `no_audio_renames_tmp_to_final_with_the_format_extension`, `gif_always_takes_the_no_audio_path_even_with_recorded_audio`, `webm_final_path_uses_the_webm_extension` - unit tests covering the pure-rename path (no ffmpeg spawn needed) for each format, including the `Gif`-with-recorded-mic-audio case.
- `both_tracks_args_cap_duration_with_t_immediately_before_the_output_path`, `single_track_args_cap_duration_with_t_immediately_before_the_output_path` - unit tests on `mux_args` (no ffmpeg spawn) asserting `-t <out_dur_ms as seconds, 3 decimals>` sits immediately before the output path, for both branches.

## mux_args

```rust
fn mux_args(tmp: &Path, tracks: &[AudioTrack], acodec: &str, out_dur_ms: u64, final_path: &Path) -> Vec<std::ffi::OsString>
```

Builds the `ffmpeg` args (appended after the shared `-y -v error`) that mux `tmp`'s video with 1 or 2 audio tracks into `final_path`. Pure - no process spawn - so the mix-filter-vs-single-map branch choice and the `-t` duration cap are unit-testable without launching ffmpeg. Private to the module; `mux` is the only caller, `run_ffmpeg` the only consumer of its output.

### Inputs

- `tmp: &Path` - the silent video input.
- `tracks: &[AudioTrack]` - 1 or 2 audio inputs (path, `add_offset` shift, linear volume). *Why a slice, not two `Option`s:* lets one function build both the both-tracks and one-track ffmpeg arg lists by branching on `tracks.len()` instead of duplicating the arg-assembly logic per branch. Must be length 1 or 2 - `mux`'s own branching guarantees this (0 tracks takes the separate rename path and never reaches here).
- `acodec: &str` - the audio encoder for `-c:a` (`format.audio_codec()`).
- `out_dur_ms: u64` - see `mux`'s own `out_dur_ms`. Formatted to 3 decimal seconds and appended as `-t <secs>` immediately before `final_path`.
- `final_path: &Path` - the output file path, appended last.

### Returns

`Vec<OsString>` - the full arg list, in ffmpeg's expected order: `-i <tmp>` first, then per-track `offset_args` + `-i <track>` (in `tracks` order), then the mix filter (`tracks.len() == 2`) or single map (`tracks.len() == 1`), then `-t <out_dur_ms as seconds>`, then `final_path`.

### Implementation

1. Push `-i <tmp>`.
2. For each track: push `offset_args(track.shift_ms)`, then `-i <track.path>`.
3. **`tracks.len() == 2`:** push `-filter_complex [1:a]volume={vol0}[m];[2:a]volume={vol1}[s];[m][s]amix=inputs=2:normalize=0[a] -map 0:v -map [a] -c:v copy -c:a {acodec}` - each track's volume filter runs BEFORE the mix, so muting one track (volume 0) silences only that track instead of the mixed output. *Why `normalize=0`:* prevents automatic loudness normalization that would alter the user's recorded audio levels.
4. **Else (`tracks.len() == 1`):** push `-map 0:v -map 1:a -c:v copy -c:a {acodec} -af volume={vol}`.
5. Push `-t <out_dur_ms as seconds, 3 decimals>` then `final_path`.

## AudioTrack

```rust
struct AudioTrack<'a> { path: &'a Path, shift_ms: i64, vol: f32 }
```

One audio input to `mux_args`: its file path, signed A/V shift (`add_offset`'s sign convention), and linear volume gain. Private to the module - `mux` constructs one `AudioTrack` per existing audio file and passes them to `mux_args` as a slice.

## offset_args

```rust
fn offset_args(shift_ms: i64) -> Vec<std::ffi::OsString>
```

Pure arg list for the same alignment `add_offset` applies (see there for the sign convention): `["-itsoffset", "<secs>"]` if positive, `["-ss", "<secs>"]` if negative, empty if zero. Shared by `add_offset` (appends onto a `Command`) and `mux_args` (appends onto its `Vec<OsString>`) so both builders emit identical args from one implementation.

## add_offset

```rust
pub(crate) fn add_offset(c: &mut Command, shift_ms: i64)
```

Appends the ffmpeg args that align one audio input to the video start. Must be called immediately BEFORE that input's `-i` (ffmpeg applies `-itsoffset`/`-ss` to whichever input follows). Positive `shift_ms` delays the track (`-itsoffset <s>`: the track started early relative to the video); negative trims its lead (`-ss <s>`: the track started late); zero emits nothing.

### Inputs

- `c: &mut Command` - the in-progress `ffmpeg` command being built. *Why mutate in place:* callers interleave this with their own `-i` args per input, so appending onto the shared builder matches the surrounding code better than returning a standalone arg list.
- `shift_ms: i64` - signed offset in milliseconds, same sign convention as `mux`'s `mic_shift_ms`/`sys_shift_ms`.

### Returns

Nothing (`()`) - mutates `c` in place.

### Why `pub(crate)`

So `preview::thumbs::ensure_preview_audio` can apply the EXACT same per-track alignment the final render uses when building the editor's mixed preview track (`preview_synced.m4a`), keeping preview playback in sync with the final export instead of drifting by the capture-warmup lead.
