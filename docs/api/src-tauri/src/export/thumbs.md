# src-tauri/src/export/thumbs.rs

Editor-timeline media: cached ffmpeg helpers for the filmstrip thumbnails, the per-source audio waveform images, and a mixed preview-audio track. All mirror `ensure_proxy` (run once, cache by output existence). The recorder's proxy is silent; these give the editor frames to scrub, waveforms to show, and sound to play.

## ensure_thumbs

```rust
#[tauri::command]
pub fn ensure_thumbs(folder: String, count: u32) -> Result<Vec<String>, String>
```

N evenly-spaced JPEG thumbnails (height 64) for the filmstrip, cached in `folder/thumbs_<count>_64/`.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates the proxy/source video and the cache dir.
- `count: u32` - number of thumbnails (clamped 8..120). *Why:* the timeline fits ~16 across its width.

### Returns

`Result<Vec<String>, String>` - the per-file paths (the frontend wraps each with `convertFileSrc`). Errors if the ffmpeg pass fails.

### Implementation

1. If `thumb_0001.jpg` already exists in the cache dir, return the existing files.
2. Else one ffmpeg pass over the re-timed proxy (`preview_720_rt.mp4`, else `video.mp4`): `-vf fps=<count>/<dur_s>,scale=-2:64 -q:v 4`, where `dur_s = load_or_seed(folder).trim.out_ms / 1000`. (NOTE: the duration basis is the trim-out; correct while `trim.in_ms == 0`, which is the current default before a trim UI exists. Reading the `_rt` proxy also yields the right thumbnail count - the raw `video.mp4` is sped up, so `fps=count/dur` over it would emit fewer than `count` frames.) The `fps` filter is best-effort about the exact count; the collect loop tolerates a frame more/less.

## ensure_waveform

```rust
#[tauri::command]
pub fn ensure_waveform(folder: String, which: String) -> Result<String, String>
```

A waveform PNG for one source (`"system"` or `"mic"`), cached as `folder/wave_<which>.png`.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path.
- `which: String` - `"system"` or `"mic"` (else an error). *Why:* selects `paths.system()` / `paths.mic()`.

### Returns

`Result<String, String>` - the waveform PNG path, or `""` when that source wasn't recorded (the track hides). Errors if the render fails.

### Implementation

ffmpeg `showwavespic=s=1180x26:colors=#6b6b86 -frames:v 1` over the wav. Cached by file existence; the PNG is written to a `tmp_sibling` and atomically renamed, so a concurrent reader (the editor opening during the post-record pre-warm) never loads a half-written image.

## ensure_preview_audio

```rust
#[tauri::command]
pub fn ensure_preview_audio(folder: String) -> Result<String, String>
```

A mixed mic+system preview-audio track (`folder/preview_audio.m4a`, AAC) so the editor can play sound (the proxy is silent).

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* locates `mic.wav` / `system.wav` + the cache file.

### Returns

`Result<String, String>` - the m4a path, or `""` when neither source exists.

### Implementation

ffmpeg `amix` of whichever of mic/system exist (single source is just encoded). Cached by existence; written to a `tmp_sibling` and atomically renamed so a mid-pre-warm reader never loads a partial track. **NOTE:** v1 mixes without the export's per-track `-itsoffset`/`-ss` alignment (see `export::audio_mux` + `exporter.rs` `shift()` / `settings.audio_offset_ms`), so editor playback can drift slightly vs the final render - exact-sync alignment is a tracked follow-up.

## prewarm

```rust
pub fn prewarm(folder: String)
```

Eagerly generates the editor's heavy media (proxy @720, 16 filmstrip thumbnails, system/mic waveforms, mixed preview-audio) so opening the editor is instant instead of transcoding on open. Called from the tail of `stop_recording` on a freshly spawned background thread - i.e. ONLY after the capture threads have joined, so it never competes with the live capture for GPU/CPU (the safe form of "render in parallel while recording"). Best-effort: each step's error is ignored (the editor's lazy `ensure_*` re-attempts on open). Runs the proxy first so the thumbnail pass reads the small proxy rather than the raw capture. The single-file media (proxy/waveforms/preview-audio) write atomically; the thumbnail directory is filled in place, so a rare mid-pre-warm open just shows fewer thumbnails for one frame and self-heals.
