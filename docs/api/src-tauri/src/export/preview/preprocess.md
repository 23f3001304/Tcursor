# src-tauri/src/export/preview/preprocess.rs

Pre-generates the editor's heavy preview media - proxy, filmstrip thumbnails, waveforms, mixed preview audio, and the `edit.json` seed - right after a recording stops, instead of lazily on editor open. Reuses the exact `ensure_*`/`load_or_seed` functions the editor's own lazy fallback calls (all `generate_once`-cached), so nothing here duplicates or re-runs their logic - the new behavior is running them eagerly, in sequence, with progress events, then marking the project preprocessed so `useEditorData` can skip its own lazy calls. Supersedes the old fire-and-forget `thumbs::prewarm` background spawn: the same sequence, now AWAITED by the frontend (with progress) instead of racing the editor's mount on a detached thread - that race was why the preview could still take a while to load right after Stop.

## DEFAULT_PROXY_HEIGHT

```rust
pub const DEFAULT_PROXY_HEIGHT: u32 = 720;
```

Proxy height generated during preprocessing - matches `Editor.tsx`'s initial `quality` state and the frontend's `DEFAULT_PROXY_HEIGHT` (`src/lib/ipc.ts`), so a freshly preprocessed project's default quality is always the one already sitting on disk.

## preprocess_project

```rust
#[tauri::command]
pub fn preprocess_project(folder: String, app: AppHandle)
```

Kicks off the full preprocessing pass for `folder` on a background thread and returns immediately.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; every step below derives its paths from it.
- `app: AppHandle` - used to `emit` progress/completion events. *Why:* the frontend awaits this command's actual work via events, not its own (near-instant) return value.

### Returns

Nothing directly - the frontend awaits completion via events, not this command's own promise. Emits `preprocess-progress` (`u32`, 0..100) after each of the 6 steps, then exactly one of `preprocess-done` (payload: the folder) or `preprocess-error` (payload: a message).

### Implementation

1. Spawn a background thread (so the command returns immediately and never blocks the Tauri event loop).
2. Inside it, run `run(&folder, on_progress)` - see below.
3. On `Ok(())`, emit `preprocess-done`; on `Err(e)`, emit `preprocess-error(e)`.

### Used by

- `src-tauri/src/lib.rs` - registered in `invoke_handler!`.
- `src/lib/ipc.ts` (`preprocessProject`) - the TS wrapper.
- `src/hud/hooks/useRecordingFlow.ts` - called right after `stopRecording`/`webcam.stop()` resolve, awaited before `onEdit` opens the editor.

## run

```rust
fn run(folder: &str, on_progress: impl Fn(u32)) -> Result<(), String>
```

The sequence itself: proxy first (so the thumbnail pass reads the small proxy rather than the raw capture - mirrors the old `prewarm` ordering), then filmstrip thumbs, system waveform, mic waveform, mixed preview audio, and the `edit.json` seed - reporting progress after each via `on_progress(step_pct(n))`.

### Returns

`Ok(())` only on FULL success, after which `manifest.preprocessed` is set to `true` and saved. Any failed step (`?` on an `ensure_*` call) short-circuits BEFORE the manifest is touched, so it stays `false` - a later editor open still falls back to its own lazy `ensure_*` (same as an un-preprocessed project).

## step_pct

```rust
fn step_pct(n: u32) -> u32
```

Pure: step `n` (1-based) of `STEP_COUNT` (6) as a 0..100 percent - its own function so the progression is unit-testable without touching ffmpeg.

### Behaviors

- `step_pct_reaches_100_at_the_last_step`
- `step_pct_is_monotonically_increasing`
- `step_pct_never_exceeds_100`
