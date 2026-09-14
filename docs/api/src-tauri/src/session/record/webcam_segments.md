# src-tauri/src/session/record/webcam_segments.rs

The camera half of mid-take source switching (2026-09-14; design: `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`, plan Task C): which file a webcam chunk appends to, and the command the HUD calls at the instant it swaps cameras.

The webcam is recorded in the webview by a `MediaRecorder` streaming 1s chunks to disk (`useWebcamRecorder` -> `append_webcam`). A `MediaRecorder` cannot change its stream, so a camera switch is a SECOND recorder writing a SECOND file. This module names that file and stamps where it starts on the recording clock; `export::preview::segments_webcam::merge_webcam_segments` merges the lot back into one `webcam.webm` at Stop, so the editor's stage and the export's `WebcamPipe` still see exactly one file.

## webcam_segment_name

```rust
pub fn webcam_segment_name(segment: Option<u32>) -> String
```

The file a webcam chunk with this segment index belongs in.

### Inputs

- `segment: Option<u32>` - the index the HUD sent with the chunk. `None` (a frontend that predates switching), `Some(0)` and `Some(1)` all mean the take's FIRST segment.

### Returns

`"webcam.webm"` for the first segment - the plain name every reader downstream already knows, which is why the first segment is never renumbered - and `"webcam_<n>.webm"` for any later one.

### Behaviors (pinned by unit tests in this file)

- `the_first_segment_keeps_the_name_every_reader_knows`: `None`, `Some(0)` and `Some(1)` are all `webcam.webm`.
- `every_later_segment_is_numbered_from_two`: `Some(2)` is `webcam_2.webm`, `Some(11)` is `webcam_11.webm`.

### Used by

- `commands::append_webcam` - the only thing that turns a chunk's `segment` into a path.
- `mark_webcam_segment` below - so the name in `sync.json` and the name on disk come from one function.

## mark_webcam_segment

```rust
#[tauri::command]
pub fn mark_webcam_segment(segment: u32, recorder: tauri::State<'_, Recorder>) -> Result<(), String>
```

Record that the take's camera just changed. The HUD calls this after the previous recorder has flushed and the new stream is live, immediately before `append_webcam` starts writing `webcam_<n>.webm`.

### Inputs

- `segment: u32` - the index of the segment about to start (2, 3...).
- `recorder` - the managed `Recorder` state.

### Returns

`Ok(())` after pushing `Segment { path: webcam_segment_name(Some(segment)), start_ms: recording_ms(clock, paused_totals) }` onto `Running.segments.webcam`.

`Err("the first webcam segment is webcam.webm and is never marked")` for `segment < 2`: `sync.json`'s `webcam_segments` lists only the EXTRA segments, and a `webcam.webm` entry there would make the merge concatenate the first segment onto itself.

`Err("not recording")` when no take is running.

### Why the recording clock

`recording_ms` (`segments.rs`) is the capture clock with every paused span removed - the clock `sync.json`'s frame times, `mic_ms` and every event stream are already on. Stamping there means a switch made after a long pause lands where the merge expects it instead of that much too late.

### Why the HUD stamps at the recorder, not at the tap

The command is called once the new device has actually produced a stream, which can be a second or more after the picker was tapped (`useSourceSwitch`'s `CAMERA_WAIT_MS`). The stamp is therefore the instant the new file starts, and the difference between the two is exactly the gap the merge fills with black.

### Registered in

`src-tauri/src/lib.rs`'s `invoke_handler!` list, one line under `commands::append_webcam`.
