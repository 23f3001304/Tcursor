# src-tauri/src/export/pipeline/audio_mux.rs

Muxes the encoded silent video with recorded microphone and/or system audio into `final.mp4`. Handles all four audio combinations (both tracks, one track, no tracks) and applies per-track A/V sync offsets before muxing.

## mux

```rust
pub fn mux(tmp: &Path, paths: &ProjectPaths, mic_shift_ms: i64, sys_shift_ms: i64) -> Result<()>
```

Combines the temporary video file with whichever audio tracks exist and writes `<project_folder>/final.mp4`.

### Inputs

- `tmp: &Path` - path to the composited silent video (written by the exporter's encoding step). *Why:* the video is encoded first, then audio is added in a single pass so the video stream is copied without re-encoding (`-c:v copy`).*
- `paths: &ProjectPaths` - project path resolver providing `paths.mic()`, `paths.system()`, and `paths.folder`. *Why:* audio track paths are project-specific and must be resolved through `ProjectPaths`, not hard-coded.*
- `mic_shift_ms: i64` - signed A/V offset for the microphone track in milliseconds. Positive = delay mic (mic started early relative to video); negative = trim mic lead. *Why:* microphone capture may start before or after video capture; this offset corrects drift measured from `sync.json`.*
- `sys_shift_ms: i64` - signed A/V offset for the system audio track. *Why:* system audio comes from a separate capture device and may have independent drift.*

### Returns

`Result<()>` - `Ok` on success. `Err` if ffmpeg exits non-zero or the no-audio rename fails.

### Implementation

1. Compute `final_path = paths.folder / "final.mp4"`.
2. Check `mic.exists()` and `system.exists()`.
3. **Both tracks:** call `run_ffmpeg` with three inputs (video, mic, system). Apply `add_offset` before each audio input. Use `-filter_complex [1:a][2:a]amix=inputs=2:normalize=0[a]` to mix at equal levels. Map video stream with `-c:v copy` and encode audio to AAC. *Why `normalize=0`:* prevents automatic loudness normalization that would alter the user's recorded audio levels.*
4. **One track:** call `run_ffmpeg` with two inputs (video + that track). Apply `add_offset`. Map with `-c:v copy -c:a aac`.
5. **No audio:** remove any existing `final.mp4`, rename `tmp` to `final.mp4` via `fs::rename`. *Why rename not copy:* avoids duplicating a potentially large video file when there is nothing to mux.*
6. On audio paths, delete `tmp` after a successful ffmpeg mux (superseded by `final.mp4`).
