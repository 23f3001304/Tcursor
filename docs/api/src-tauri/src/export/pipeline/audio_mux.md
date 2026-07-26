# src-tauri/src/export/pipeline/audio_mux.rs

Muxes the encoded silent video with recorded microphone and/or system audio into `final.<ext>` (`<ext>` from the export's `Format`). Handles all four audio combinations (both tracks, one track, no tracks), applies per-track A/V sync offsets, applies per-track volume gain before muxing, and picks the right audio codec for the container.

## mux

```rust
pub fn mux(tmp: &Path, paths: &ProjectPaths, format: Format, mic_shift_ms: i64, sys_shift_ms: i64, mic_vol: f32, sys_vol: f32) -> Result<()>
```

Combines the temporary video file with whichever audio tracks exist and writes `<project_folder>/final.<ext>`.

### Inputs

- `tmp: &Path` - path to the composited silent video (written by the exporter's encoding step). *Why:* the video is encoded first, then audio is added in a single pass so the video stream is copied without re-encoding (`-c:v copy`).*
- `paths: &ProjectPaths` - project path resolver providing `paths.mic()`, `paths.system()`, and `paths.folder`. *Why:* audio track paths are project-specific and must be resolved through `ProjectPaths`, not hard-coded.*
- `format: Format` (`export::settings::Format`) - the export's container/codec choice. *Why:* determines the final filename's extension (`format.extension()`), whether audio is even attempted (`format.supports_audio()` - `false` for `Gif`), and which audio encoder to use (`format.audio_codec()` - `"aac"` for `Mp4`, `"libopus"` for `WebM`).*
- `mic_shift_ms: i64` - signed A/V offset for the microphone track in milliseconds. Positive = delay mic (mic started early relative to video); negative = trim mic lead. *Why:* microphone capture may start before or after video capture; this offset corrects drift measured from `sync.json`.*
- `sys_shift_ms: i64` - signed A/V offset for the system audio track. *Why:* system audio comes from a separate capture device and may have independent drift.*
- `mic_vol: f32` - linear gain multiplier applied to the mic track (`Settings.audio_mic_volume`; 0 = muted, 1 = unchanged, up to 1.5). *Why linear rather than dB:* matches ffmpeg's `volume` filter's default unit and the 0..150% slider in `AudioPanel`, so no conversion is needed at either end.*
- `sys_vol: f32` - linear gain multiplier applied to the system-audio track (`Settings.audio_sys_volume`). Same range as `mic_vol`.*

### Returns

`Result<()>` - `Ok` on success. `Err` if ffmpeg exits non-zero or the no-audio rename fails.

### Implementation

1. Compute `final_path = paths.folder / "final.<ext>"` (`<ext>` from `format.extension()`).
2. Check `mic.exists()` and `system.exists()`, both additionally gated on `format.supports_audio()` - so `Gif` always behaves as if neither track exists, regardless of what was actually recorded.
3. **Both tracks:** call `run_ffmpeg` with three inputs (video, mic, system). Apply `add_offset` before each audio input. Use `-filter_complex [1:a]volume={mic_vol}[m];[2:a]volume={sys_vol}[s];[m][s]amix=inputs=2:normalize=0[a]` - each track's volume filter runs BEFORE the mix, so muting one track (volume 0) silences only that track instead of the mixed output. Map video stream with `-c:v copy` and encode audio with `format.audio_codec()`. *Why `normalize=0`:* prevents automatic loudness normalization that would alter the user's recorded audio levels.*
4. **One track:** call `run_ffmpeg` with two inputs (video + that track). Apply `add_offset`. Map with `-c:v copy -c:a {format.audio_codec()} -af volume={vol}` (that track's own gain).
5. **No audio** (nothing recorded, or `format` can't carry it at all - `Gif`): remove any existing `final.<ext>`, rename `tmp` to `final.<ext>` via `fs::rename`. *Why rename not copy:* avoids duplicating a potentially large video file when there is nothing to mux. No volume filter applies (nothing to mux).*
6. On audio paths, delete `tmp` after a successful ffmpeg mux (superseded by `final.<ext>`).

### Behaviors worth knowing

- `no_audio_renames_tmp_to_final_with_the_format_extension`, `gif_always_takes_the_no_audio_path_even_with_recorded_audio`, `webm_final_path_uses_the_webm_extension` - unit tests covering the pure-rename path (no ffmpeg spawn needed) for each format, including the `Gif`-with-recorded-mic-audio case.
