# src-tauri/src/export/preview/segments_audio.rs

The microphone half of mid-take source switching (2026-09-14; design: `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`, plan Task M step 3). Every `switch_mic` ends one WAV and opens the next, so a take with two switches leaves `mic.wav`, `mic_2.wav` and `mic_3.wav` on disk plus, in `sync.json`'s `mic_segments`, the recording-clock instant each extra one started at. This module runs once from `preprocess::essential`, before the editor opens, and leaves exactly the single continuous `mic.wav` that was there before switching existed - so `pipeline::timeline`, `pipeline::audio_mux`, the preview mux and every waveform keep reading one file and need no idea that a switch happened.

Silence between segments (the ~50ms of device teardown and setup, or a whole stretch with the mic switched off) is what the delays produce, so every later word stays where it was spoken. The originals are moved to `<folder>/segments/`, never deleted: a re-merge is always possible, and a bad merge can never cost a take its narration.

## MicPart

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct MicPart { pub path: PathBuf, pub delay_ms: u64 }
```

One WAV going into the merge: where it is, and how far into the merged track it starts. `delay_ms` is measured from the merged track's own origin, so the first part is always `0`.

## merge_mic_segments

```rust
pub fn merge_mic_segments(paths: &ProjectPaths, sync: &SyncLog) -> Result<(), String>
```

Merge every extra mic segment `sync` lists into `mic.wav`. **The entry point** - the only item here the wiring in `preprocess::essential` calls.

### Inputs

- `paths: &ProjectPaths` - the project folder. `paths.mic()` is both an input (the first segment) and the destination; extra segments are resolved as `paths.folder.join(&segment.path)`.
- `sync: &SyncLog` - the take's `sync.json`, already loaded by the caller. Read: `mic_segments` (which extras exist and when each started), `mic_ms` (where `mic.wav` starts) and `frames` (only as the origin fallback, below).

### Returns

`Ok(())` on success, and also on every no-op. `Err(String)` when the first segment's header is unreadable, ffmpeg fails or will not spawn, or the final rename fails. A failure is the caller's to log: the take is never lost, because the first segment is still on disk (put back, if the rename was what failed) and still the file the editor opens.

### When it does nothing at all

- `sync.mic_segments` is empty - the ordinary take, with no mid-take switch. Returns before touching the filesystem.
- No part has a file on disk (every switch was a mute, or every device failed to open).
- Exactly one part, which is `mic.wav` itself at delay 0 - nothing to delay and nothing to mix, so no re-encode. **This is also what makes a second pass a no-op:** after a successful merge the extras live in `segments/`, so the next call finds only `mic.wav` and returns.

### Implementation

1. Build the input list with `parts_of` and take the no-op exits above.
2. Read the FIRST part's sample rate and channel count with `wav_spec`; that spec is the merged track's, and every other part is resampled to it.
3. Build the command with `merge_args` and run it into `proc::tmp_sibling(&paths.mic())` (same folder, `.wav` preserved so ffmpeg still infers the container). A failure deletes the temp and returns.
4. Move every original into `<folder>/segments/` **before** the rename. *Why that order:* the merged file takes `mic.wav`'s name, so the first segment has to be out of the way, and it must land somewhere recoverable rather than be overwritten. If the final rename then fails, `segments/mic.wav` is put straight back, so the take keeps its first segment.
5. `rename(tmp, mic.wav)` - atomic on the same volume, so a reader never sees a half-written WAV.

### Behaviors

- `a_take_with_no_switch_is_left_exactly_as_it_was`: with an empty `mic_segments` the bytes of `mic.wav` are unchanged and no `segments/` folder appears.
- `two_generated_wavs_merge_into_one_track_with_the_gap_between_them` (real ffmpeg, skipped when none is on PATH): two generated 100 ms WAVs logged 150 ms apart merge to a 250 ms `mic.wav` (probed with `pipeline::ffio::probe_duration`, +-5 ms), both originals are kept under `segments/`, and a second call changes nothing.

## merge_args

```rust
pub fn merge_args(parts: &[MicPart], rate: u32, channels: u16, out: &Path) -> Vec<OsString>
```

The ffmpeg args, appended after the shared `-y -v error`, that merge `parts` into `out`. Pure (no spawn, no filesystem), so the exact command is pinned by tests instead of discovered by ear on a real recording - the same split `pipeline::audio_mux::mux_args` uses.

### The filter graph

Each input `i` becomes one branch:

```
[i:a]aresample=<rate>,aformat=channel_layouts=<layout>[,adelay=<d>|<d>][s<i>]
```

and, with more than one input, the branches are summed:

```
[s0][s1]...amix=inputs=<n>:normalize=0:dropout_transition=0[a]
```

- *Why `aresample` + `aformat` on every branch, including the first:* `amix` refuses inputs that disagree on rate or layout, and a mid-take switch routinely lands on a device with another sample rate (a 44.1 kHz stereo headset after a 48 kHz mono array). `rate`/`channels` are the FIRST part's own spec, so the merged track is bit-for-bit the format the take started in.
- *Why `adelay` and not `concat` with `anullsrc` gaps:* the delays are absolute positions on the recording clock, so each segment lands where it was recorded whatever the previous one's real duration turned out to be - a concat would accumulate every rounding error and every dropped sample. A zero delay emits no `adelay` at all, so the first branch stays the plain conversion it is. `adelay` takes one value per channel, hence the `<d>|<d>` repetition for stereo.
- *Why `normalize=0`:* the default scales every sample by `1/n`, which would quietly halve a two-segment take. *Why `dropout_transition=0`:* otherwise amix fades around the instant a segment ends.
- With exactly one input there is no `amix`; the single branch is labelled `[a]` directly. That is the "mic switched ON mid-take" case, where the whole merge is a lead-in silence.

Output flags are `-map [a] -c:a pcm_s16le` - the format `audio::wav_writer` already writes, so nothing about `mic.wav` changes except its length.

### Behaviors

- `one_segment_is_delayed_into_place_without_a_mix`: a single part at 1500 ms produces the full arg list with no `amix`.
- `two_segments_mix_with_the_second_delayed_by_its_gap`: input order, a first branch with no `adelay`, and the unnormalised mix.
- `three_segments_are_all_forced_to_the_first_ones_rate_and_layout`: with a 44100/stereo first segment, all three branches carry `aresample=44100,aformat=channel_layouts=stereo` and paired `adelay` values.

## parts_of

```rust
fn parts_of(paths: &ProjectPaths, sync: &SyncLog) -> Vec<MicPart>
```

The merge's inputs in recording order, with each delay resolved.

A segment with no file on disk is skipped: a switch TO no mic logs its boundary but writes nothing, and a device that would not open has had its header-only WAV removed by `recorder_threads::spawn_mic_thread` (so that `pipeline::timeline` never mistakes it for real audio).

**The origin every delay is measured from** is `sync.mic_ms` - where `sync.json` places `mic.wav` - falling back to `sync.frames.first()` when the take started with the mic OFF, because then there is no first segment and no `mic_ms`. *Why the first frame is the right fallback:* with `mic_ms == None`, `pipeline::audio_shift_ms` muxes the track at a zero shift, i.e. starting at the video's own frame 0. Measuring from `frames[0]` is what makes the merged file's lead-in silence line the first spoken word up with the picture.

Both clocks agree by construction: `segments::recording_ms` is the capture clock with paused spans removed, which is the clock `frames` and `mic_ms` are already on, and the WAVs themselves contain no paused audio (`CpalMic::open` drops samples while `paused`).

### Behaviors

- `delays_are_measured_from_the_first_segments_own_start`: with `mic_ms = 1000`, a segment at 7000 gets `delay_ms = 6000`, and a listed segment whose file is absent is not an input.
- `a_take_that_started_muted_measures_from_the_videos_first_frame`: no `mic.wav`, no `mic_ms`, `frames = [500, ...]`, a segment at 4500 gets `delay_ms = 4000`.

## wav_spec

```rust
fn wav_spec(path: &Path) -> Result<(u32, u16), String>
```

A WAV's sample rate and channel count, read straight from its own header with `hound` rather than probed through an `ffprobe` subprocess - the merge only needs the first segment's spec, and `audio::wav_writer` already writes these files with the same library.

## layout

```rust
fn layout(channels: u16) -> String
```

ffmpeg's `channel_layouts=` name for a channel count: `mono`, `stereo`, or the `<n>c` form. Every real input device is mono or stereo; `<n>c` covers the rest rather than guessing a speaker arrangement for a layout nobody records a narration on.

## run

```rust
fn run(args: &[OsString]) -> Result<(), String>
```

`ffmpeg -y -v error <args>` with both streams silenced. Uses `proc::ffcmd_bg`, not `ffcmd`: this is preprocess work running while the editor is opening, so it takes BELOW_NORMAL_PRIORITY_CLASS like every other pre-warm pass and leaves the UI thread schedulable.
