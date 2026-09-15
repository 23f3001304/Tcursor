# src-tauri/src/export/preview/thumbs.rs

Editor-timeline media: cached ffmpeg helpers for the filmstrip thumbnails, the per-source audio waveform images, and a mixed preview-audio track. All mirror `ensure_proxy` (run once, cache by output existence) and wrap their ffmpeg pass in `process::proc::generate_once` (so the post-record preprocessing pass, `export::preview::preprocess::preprocess_project`, and the editor's own lazy `ensure_*` never transcode the same file twice or storm the CPU with concurrent passes right as the editor opens) run via `ffcmd_bg` (below-normal priority, so the one serialized multi-threaded pass yields to the UI instead of freezing it). The recorder's proxy is silent; these give the editor frames to scrub, waveforms to show, and sound to play.

**Off the main thread (Task 41).** All three IPC commands are `async fn`; each wraps a `_blocking` sibling (same body the sync command used to run) in `tauri::async_runtime::spawn_blocking`. Same freeze mechanism `ai::commands` fixed for Task 40: a non-`async` `#[tauri::command] fn` runs INLINE on the thread that received the IPC message (the app's main/UI thread), so a sync version of these would freeze the window for the whole ffmpeg pass - a proxy transcode (`ensure_proxy`, `preview_track.rs`) can run for seconds on a project OPEN. `preprocess::run` calls the `_blocking` functions directly (not the `async` commands) since it already runs off-thread on its own `std::thread::spawn`, outside any `.await` context.

## FILMSTRIP_COUNT

```rust
pub(crate) const FILMSTRIP_COUNT: u32 = 9;
pub(crate) const FILMSTRIP_HEIGHT: u32 = 80;
```

The filmstrip the editor actually draws, mirrored from `src/editor/timeline/model/filmstripPlan.ts` (which DERIVES the pair from the lane's drawn height and the editor window's usual width - see `filmstripPlan.md`). `preprocess::rest` pre-renders with exactly these, so the background pass fills the very `thumbs_9_80` dir the editor then asks for instead of leaving it a second ffmpeg run. A unit test asserts the pair still names that dir; keep the two sides in step.

## FILMSTRIP_HEIGHT

See `FILMSTRIP_COUNT` above - the two are one decision and are declared together.

## thumbs_spec

```rust
pub(crate) fn thumbs_spec(count: u32, height: u32) -> (u32, u32, String)
```

The clamped count, the clamped EVEN height, and the cache dir name the pair owns (`thumbs_<count>_<height>`) - the one place those three are derived, so the ffmpeg args, the collect loop and the cache key can never disagree.

### Inputs (what, and why it is needed)

- `count: u32` - requested thumbnails, clamped 8..120. *Why:* a filmstrip needs at least a few tiles, and past ~120 the pass costs more than the strip is worth.
- `height: u32` - requested pixel height, clamped 16..240 and rounded DOWN to even. *Why:* `scale=-2:h` only guarantees an even WIDTH; an odd height leaves the JPEG's 4:2:0 chroma plane half a line short.

### Returns

`(count, height, dir_name)`. The height is part of the dir name (it used to be a hard-coded `64`) so a taller lane re-renders its thumbnails at the size it draws them instead of silently upscaling a cached smaller set.

## ensure_thumbs

```rust
#[tauri::command]
pub async fn ensure_thumbs(folder: String, count: u32, height: u32) -> Result<Vec<String>, String>
```

Tauri IPC command. `spawn_blocking(ensure_thumbs_blocking)`, `.await`ed, join failure mapped to `Err(String)`.

## ensure_thumbs_blocking

```rust
pub(crate) fn ensure_thumbs_blocking(folder: String, count: u32, height: u32) -> Result<Vec<String>, String>
```

N evenly-spaced JPEG thumbnails at `height` px for the filmstrip, cached in `folder/thumbs_<count>_<height>/`.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates the proxy/source video and the cache dir.
- `count: u32` / `height: u32` - the filmstrip's tile count and pixel height, both through `thumbs_spec`. *Why:* the lane is 80px tall and fits nine 16:9 tiles across the editor's usual width (`filmstripPlan.ts`); generating at the drawn height is what keeps a tile from being an upscaled smaller JPEG.

### Returns

`Result<Vec<String>, String>` - the per-file paths (the frontend wraps each with `convertFileSrc`). Errors if the ffmpeg pass fails.

### Implementation

1. If `thumb_0001.jpg` already exists in the cache dir, return the existing files.
2. Else one ffmpeg pass over the re-timed proxy (`preview_720_rt.mp4`, else `video.mp4`): `-vf fps=<count>/<dur_s>,scale=-2:<height> -q:v 4`, with `-skip_frame nokey` when the source is the proxy - it carries a keyframe every second (`preview_track`'s `-g 60`), so decoding only keyframes gives the `fps` filter a sample per second to pick from at about half the cost (5.0s -> 2.9s on a 5-minute 1080p60 take); the raw capture's keyframes are seconds apart, so it is still decoded whole - where `dur_s = edit::seed::true_duration_ms(paths) / 1000` - the recording's TRUE full duration, not `trim.out_ms` (a sub-range once a user actually trims): the filmstrip spans the whole scrubbable timeline regardless of trim. Reading the `_rt` proxy also yields the right thumbnail count - the raw `video.mp4` is sped up, so `fps=count/dur` over it would emit fewer than `count` frames. The `fps` filter is best-effort about the exact count; the collect loop tolerates a frame more/less.

## ensure_waveform

```rust
#[tauri::command]
pub async fn ensure_waveform(folder: String, which: String) -> Result<String, String>
```

Tauri IPC command. `spawn_blocking(ensure_waveform_blocking)`, `.await`ed, join failure mapped to `Err(String)`.

## ensure_waveform_blocking

```rust
pub(crate) fn ensure_waveform_blocking(folder: String, which: String) -> Result<String, String>
```

A waveform PNG for one source (`"system"` or `"mic"`), cached as `folder/wf_<which>.png`. An absent WAV, or one that is header-only (`wav_is_empty`: 44 bytes, a source that was on but never delivered a sample, such as a silent loopback), returns `Ok("")` - "no audio", never a render attempt, because `showwavespic` fails on it and until 2026-09-14 that failure took the editor's other lane down with it.

## wav_is_empty

```rust
pub(crate) fn wav_is_empty(wav: &Path) -> bool
```

True for a WAV that holds no samples: nothing past the 44-byte RIFF/fmt/data header (or a file whose metadata cannot be read). Test: `a_header_only_wav_yields_no_waveform_instead_of_an_error`.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path.
- `which: String` - `"system"` or `"mic"` (else an error). *Why:* selects `paths.system()` / `paths.mic()`.

### Returns

`Result<String, String>` - the waveform PNG path, or `""` when that source wasn't recorded (the track hides). Errors if the render fails.

### Implementation

ffmpeg `dynaudnorm,showwavespic=s=1180x26:colors=#6b6b86:scale=sqrt -frames:v 1` over the wav - `dynaudnorm` normalizes loudness and the `sqrt` scale emphasizes low amplitudes, so a quiet mic shows a visible waveform instead of a flat, invisible line (the export mic is audible regardless). Cached by file existence (the `wf_` prefix invalidates older flat `wave_*` caches); the PNG is written to a `tmp_sibling` and atomically renamed, so a concurrent reader never loads a half-written image.

## ensure_preview_audio

```rust
#[tauri::command]
pub async fn ensure_preview_audio(folder: String) -> Result<String, String>
```

Tauri IPC command. `spawn_blocking(ensure_preview_audio_blocking)`, `.await`ed, join failure mapped to `Err(String)`.

## ensure_preview_audio_blocking

```rust
pub(crate) fn ensure_preview_audio_blocking(folder: String) -> Result<String, String>
```

A mixed mic+system preview-audio track (`folder/preview_synced.m4a`, AAC) so the editor can play sound (the proxy is silent). Each track is shifted to the video start with the SAME per-track `-itsoffset`/`-ss` alignment the final render uses (`audio_mux::add_offset`), so preview playback stays in sync with the (re-timed) proxy instead of drifting by the capture-warmup lead.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates `mic.wav` / `system.wav` + the cache file.

### Returns

`Result<String, String>` - the m4a path, or `""` when neither source exists.

### Implementation

1. Return `""` immediately if neither `mic.wav` nor `system.wav` exists - no cache file is created.
2. Compute `(mic_shift, sys_shift)` via `preview_audio_shifts` (below), BEFORE entering the cache check, so a cache hit still returns the right path with no extra work.
3. Inside `generate_once` (cached by existence of `preview_synced.m4a`): for each track that exists, call `audio_mux::add_offset` (the same `pub(crate)` helper `audio_mux::mux` uses for the final render) right before that track's `-i`, so the alignment matches the export exactly. With both tracks, mix via `[0:a][1:a]amix=inputs=2:normalize=0[a]` (`normalize=0` for the same reason as the final mux - don't alter recorded levels); with only one, it's just encoded straight through. Encode `-c:a aac`, write to a `tmp_sibling`, then atomically rename onto `preview_synced.m4a` so a mid-preprocess reader never loads a partial track.

The `preview_synced.` name (vs the old `preview_audio.`) invalidates stale caches written before this alignment fix existed.

*Formerly also home to `prewarm`, a fire-and-forget background-thread caller of these same functions spawned from the tail of `stop_recording`. `export::preview::preprocess::preprocess_project` supersedes it: the same sequence (plus the `edit.json` seed and the manifest flip), now AWAITED by the frontend with progress instead of racing the editor's mount on a detached thread.*

## preview_audio_shifts

```rust
fn preview_audio_shifts(paths: &ProjectPaths) -> (i64, i64)
```

Per-track mic/system shift (ms) to align preview audio to the video's frame 0. It CALLS the same seam the export mux uses (`pipeline::audio_shift_ms`) rather than re-deriving the formula, passing a zero trim: the preview always plays the whole clip, so trim is a playback clamp the frontend applies rather than a mux-time shift, and only the export has a (frame-floored) trim-in to subtract.

### Inputs

- `paths: &ProjectPaths` - project folder. *Why:* loads `events.json` and the edit doc to rebuild the same `Timeline` the export uses.

### Returns

`(i64, i64)` - `(mic_shift_ms, sys_shift_ms)`. Each is `track_start - video_start` (`0` if that track wasn't recorded); the mic value additionally adds the user's manual `audio_offset_ms` nudge. Returns `(0, 0)` if `events.json` can't be loaded, so a missing/corrupt event log falls back to unshifted preview audio instead of failing `ensure_preview_audio` outright.

### Implementation

1. Load the event log via `EventLog::load`; return `(0, 0)` on failure.
2. Rebuild the `Timeline` via `build_timeline(paths, &log, 60)` - the same call the exporter makes, at a fixed 60 fallback fps (preview alignment only needs the derived mic/system start offsets, not the true capture rate).
3. `vs` = the timeline's first frame timestamp (`video_start`). `mic = audio_shift_ms(tl.mic_ms, vs, 0) + audio_offset_ms` (from `edit::seed::load_or_seed(paths).settings`); `sys = audio_shift_ms(tl.system_ms, vs, 0)`.
