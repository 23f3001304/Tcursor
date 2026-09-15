# src-tauri/src/export/preview/preprocess.rs

Pre-generates the editor's heavy preview media right after a recording stops, instead of lazily on editor open, reusing the exact `ensure_*_blocking`/`load_or_seed` functions the editor's own lazy `ensure_*` IPC commands call (all `generate_once`-cached), so nothing here duplicates their logic.

**What the editor waits for (2026-09-14).** Only the proxy. The owner's "on long recordings it takes very long to come into the editor" was this pass running all six steps in series before `preprocess-done`: on a 5-minute 1080p60 take the proxy transcode alone was ~13s (CPU decode plus x264 at `veryfast`), the filmstrip another ~5s (it decoded the whole proxy), then the waveforms and the audio mix. Now `essential` builds the proxy (a third quicker at `ultrafast`, reporting its own progress) and marks the project preprocessed, `preprocess-done` fires, and `rest` builds the filmstrip (keyframes only), the two waveforms, the mixed preview audio and the `edit.json` seed on the same thread afterwards. The editor requests all of those lazily anyway and gets each one as it lands; `generate_once`'s lock serialises a lazy request against the background pass, so no artifact is ever built twice.

Both call the `_blocking` variants directly rather than the `#[tauri::command] async fn` wrappers (Task 41 - the commands themselves moved off the main thread via `spawn_blocking`): this already runs on its own `std::thread`, off the Tokio runtime, so there is no `.await` context to call the `async` wrappers from.

## DEFAULT_PROXY_HEIGHT

```rust
pub const DEFAULT_PROXY_HEIGHT: u32 = 720;
```

Proxy height generated during preprocessing - matches `Editor.tsx`'s initial `quality` state and the frontend's `DEFAULT_PROXY_HEIGHT` (`src/shared/ipc.ts`), so a freshly preprocessed project's default quality is always the one already sitting on disk.

## preprocess_project

```rust
#[tauri::command]
pub fn preprocess_project(folder: String, app: AppHandle)
```

Kicks off preprocessing for `folder` on a background thread and returns immediately.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; every step derives its paths from it.
- `app: AppHandle` - used to `emit` progress/completion events. *Why:* the frontend awaits the editor-ready point via events, not this command's own (near-instant) return value.

### Returns

Nothing directly. Emits `preprocess-progress` (`u32`, 0..100 - the proxy transcode's own progress, from `preview_track::proxy_pct`, then 100 once the file is in place) while the proxy builds, then exactly one of `preprocess-done` (payload: the folder - the editor may open now) or `preprocess-error` (payload: a message). The remaining artifacts keep building after `done`; nothing is emitted for them.

### Implementation

1. Spawn a background thread (so the command returns immediately and never blocks the Tauri event loop).
2. Inside it, `essential(&folder, on_progress)`.
3. On `Ok(())`, emit `preprocess-done` and then run `rest(&folder)`; on `Err(e)`, emit `preprocess-error(e)`.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/shared/ipc.ts` (`preprocessProject`) - the TS wrapper.
- `src/hud/hooks/useRecordingFlow.ts` - called right after `stopRecording`/`webcam.stop()` resolve, awaited (via `preprocess-done`) before `onEdit` opens the editor; `savePct` is the take pill's progress line.

## essential

```rust
fn essential(folder: &str, on_progress: impl Fn(u32)) -> Result<(), String>
```

The one artifact the editor cannot open without: the default-quality proxy (`ensure_proxy_with_progress`), with `on_progress` forwarded, then `on_progress(100)`.

Before the proxy, the mid-take source switches (2026-09-14): `SyncLog::load` (a missing or unreadable `sync.json` reads as the default, empty log), then `segments_audio::merge_mic_segments`, which folds the extra `mic_<n>.wav` segments back into the one `mic.wav` that `rest` builds the mic waveform and the mixed preview audio from, then `segments_webcam::merge_webcam_segments`, which concatenates the `webcam_<n>.webm` segments (black over the gaps) back into the one `webcam.webm` the stage and the export read. A take with no switch lists nothing and nothing is touched. A merge error is logged (`[PREPROCESS] mic merge:` / `[PREPROCESS] webcam merge:`) and never fails the take: the first segment stands alone, exactly as it was recorded.

### Returns

`Ok(())` after `manifest.preprocessed` is set to `true` and saved - which `useEditorData` reads as "the default proxy is on disk, point straight at it" (`planProxySrc`'s `known` branch), so the editor never shows the raw capture first. A proxy failure returns the error BEFORE the manifest is touched, so it stays `false` and the editor's own lazy `ensure_proxy` gets another go.

## rest

```rust
fn rest(folder: &str)
```

Everything the editor can also fetch for itself, built after it has opened: filmstrip thumbs (`ensure_thumbs_blocking`, from the proxy, keyframes only, at `thumbs::FILMSTRIP_COUNT`/`FILMSTRIP_HEIGHT` - the editor's own request verbatim, since a different count or height here would fill a `thumbs_<count>_<height>` dir the editor never reads), the system and mic waveforms, the mixed preview audio, and the `edit.json` seed (`load_or_seed`). Best-effort (`let _ =`): a failure here just leaves that one artifact to the editor's lazy `ensure_*` fallback.

### Behaviors (unit tests, on `preview_track::proxy_pct`)

- `a_progress_line_maps_output_time_onto_the_real_duration`: `out_time_us=30000000` against 120s is 25.
- `the_transcode_never_reports_done_itself_and_ignores_the_rest_of_the_stream`: output past the duration caps at 99; `frame=`, `progress=` and an unparsable value are `None`.
