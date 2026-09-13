# src-tauri/src/session/record/recorder_stop.rs

The Stop half of the recorder command surface, split from `recorder.rs` (which owns the session state and Start/Pause/Resume) so both stay under the 200-line cap.

## RecordingResult

```rust
#[derive(Serialize)]
pub struct RecordingResult { pub folder: String, pub frames: u64 }
```

Returned by `stop_recording` to the frontend.

- `folder: String` - absolute path to the project directory. *Why:* the frontend passes this path back to `preprocess_project`, `export_project` and the editor commands.
- `frames: u64` - total successfully encoded video frames. *Why:* informational for the UI; also useful when diagnosing a drop in frame count.

## StoppingGuard

```rust
struct StoppingGuard<'a>(&'a Recorder);
```

Clears `Recorder::stopping` when `stop_blocking` leaves scope, however it leaves. *Why a guard and not a store at the end:* a panic anywhere in the teardown would otherwise leave the flag stuck on, and a stuck flag means every later `start_recording` returns `"already recording"` - the Record button dead for the rest of the session. Its `Drop` takes the `inner` lock before clearing, so "a start either sees `stopping`, or sees a fully torn-down recorder" is an ordering guarantee rather than an accident of timing.

## stop_recording

```rust
#[tauri::command]
pub async fn stop_recording(app: tauri::AppHandle) -> Result<RecordingResult, String>
```

Signals all threads to stop, joins them in dependency order, persists input data, `sync.json`, and `project.tcursor`, and returns the `RecordingResult`. A thin `async` wrapper: the work is `stop_blocking`, below.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`, the same conversion `ai::commands`, `thumbs.rs` and `preview_track.rs` already had. As a sync `#[tauri::command] fn` this ran on the whole app's main thread while it joined the mic and system-audio threads (each a 50 ms poll loop plus a WAV-header finalize), gzip-compressed and wrote the entire mouse-event log, and then joined the video pipeline - which waits for the encoder to close its pipe and write the `moov` atom of a potentially multi-GB MP4. On a long 4K recording that froze the HUD outright: no repaint, no "Saving..." spinner motion, no input, for the whole finalize.

`recorder: tauri::State<'_, Recorder>` is not in the signature because a `State<'_, T>` cannot cross into `spawn_blocking` (its lifetime is not `'static`); `stop_blocking` re-derives it from the `AppHandle` instead. Both were injected params, so the JS call (`invoke("stop_recording")`) is unchanged.

### Inputs

- `app: tauri::AppHandle` - resolves the `Recorder` managed state inside the blocking closure, and the main window whose icon is swapped back to normal. The icon swap is brand flair only - never fails the command.

### Returns

`Ok(RecordingResult)` with the project folder and frame count. `Err(String)` if the video thread panicked, the video finalize failed, or the `spawn_blocking` task itself failed to join. **An `Err` no longer means the take is lost:** the project folder is fully written first (see step 5), so the error is a report, not a discard.

## stop_blocking

```rust
fn stop_blocking(app: &tauri::AppHandle) -> Result<RecordingResult, String>
```

The whole body of `stop_recording`, split out so the command itself is just the `spawn_blocking` hop. Runs on a blocking-pool thread. Every handle it touches is thread-agnostic by construction: `MouseTracker::stop` and `CaptureControl::stop` post `WM_QUIT` to a *stored* thread id and then join, `KeyboardTracker`/`CursorTypeTracker` are an atomic flag plus a join, and `windows-capture`'s `VideoEncoder` is declared `Send` and does its muxing on its own transcode thread.

### Implementation

1. Under ONE `inner` lock acquisition: `take` the `Running` (returning `Err("not recording")` if `None`) and set `recorder.stopping = true`. *Why `take`:* consumes the `Running`, making the state `None`. *Why the flag, set under the same lock:* everything below runs outside the lock (holding it across a multi-GB finalize would block Pause/Resume on the main thread), and until `save_inputs` has run the finished take still owns the process-global mouse-hook sink - see `Recorder::stopping` for what a start racing into that window does.
2. Call `brand_icon::set_recording(&app, false)` - only reached once step 1 confirms a recording was actually taken, so a redundant Stop (already-idle) never touches the icon.
3. Set `stop = true` (SeqCst) - signals the audio threads (the video pipeline is stopped below) - then join `mic_thread` and `system_thread`. *Why audio first:* lightweight (50ms loop), they finish quickly.
4. Build a `ProjectPaths` from `running.folder` (the captured cursor layer is several files, so `save_inputs` takes the paths rather than one more `&Path` argument) and call `save_inputs`. *Why before the video stop:* these are cheap and must survive a video finalize failure.
5. `running.video.stop_and_collect()` -> `VideoStopped` (GPU: end capture + `encoder.finish()`; ffmpeg: set halt + WM_QUIT to unblock the WGC thread + join), then `recorder_threads::save_session_files` with `stopped.frame_ts`, `events_ms`, the atomic audio start times (0 treated as absent) and `running.screen`. *Why the video error is carried in the struct instead of propagating here:* it used to propagate with `?`, which skipped `sync.json`, `project.tcursor` and the recents entry entirely - so an encoder/mux tail failure (disk full on a long take) left a folder with no manifest, which `open_project`'s `*.tcursor` filter cannot even select, and no `sync.json`, so `build_timeline` would have synthesised a timeline and muxed the mic ~800 ms early. The frames that were recorded are on disk either way, so they are written either way.
6. `StoppingGuard` drops, clearing `stopping`. *Why only here:* the old take is fully detached from the global input hooks only once `save_inputs` and the video stop have both run.
7. Return `Err(stopped.error)` if the video finalize failed (the folder is still saved; the HUD shows what went wrong), else `Ok(RecordingResult { folder, frames })`.
