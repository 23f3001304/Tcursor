# src-tauri/src/asr/commands.rs

The two IPC commands the Captions panel needs before it can transcribe anything: what models exist and whether they are on disk, and "fetch this one".

The download runs on its own thread and reports through events, the same shape `preprocess_project` uses, so the frontend never sits on a promise for several minutes. Nothing here contains download logic; it is the Tauri wrapper around `asr::download`.

## ModelDto

```rust
#[derive(serde::Serialize)]
pub struct ModelDto { pub id: String, pub label: String, pub bytes: u64, pub installed: bool, pub multilingual: bool }
```

One row of `MODELS` as the frontend sees it. `ModelSpec`'s `file` and `sha256` are deliberately NOT here: the frontend has no business knowing a filename or a digest, and every operation it can perform is keyed by `id`. Mirrored by `WhisperModelDto` in `src/shared/ipc.ts`.

## DownloadProgress

```rust
#[derive(serde::Serialize, Clone)]
pub struct DownloadProgress { pub id: String, pub done: u64, pub total: u64 }
```

The `asr-download-progress` payload. `total` is the table's byte count, not a `Content-Length`, so a resumed download's bar starts where it left off instead of at zero. Carries `id` because a progress event is meaningless without knowing which row it belongs to.

## DOWNLOADING

```rust
static DOWNLOADING: Mutex<Vec<String>>
```

Ids with a download in flight, process-wide.

A `Vec` rather than the obvious `HashSet` only because `Vec::new()` is `const` and `HashSet::new()` is not, so this needs no `OnceLock` or lazy init; the set is at most `MODELS.len()` long, where a linear scan is faster than hashing anyway.

Both accessors use `unwrap_or_else(|e| e.into_inner())` on the lock: a poisoned mutex here means a previous download thread panicked, and refusing every future download because of that would be a worse outcome than continuing with a list that is at most one stale entry wrong.

## claim

```rust
fn claim(id: &str) -> bool
```

Take the download slot for `id`, or report that someone already has it. This is what makes a double-click harmless: two writers appending to one `.part` would interleave bytes and produce a file that fails its digest after the full download.

## release

```rust
fn release(id: &str)
```

Give the slot back. Called on both the success and the failure path, so a failed download can be retried immediately.

## whisper_models

```rust
#[tauri::command] pub fn whisper_models() -> Vec<ModelDto>
```

Every model the app knows about, with whether its file is already on disk.

Sync and cheap - a four-row table walk plus one `exists()` per row - so there is no `spawn_blocking` hop. `installed` is re-resolved on every call rather than cached, which is what lets the panel simply call this again after `asr-download-done` instead of patching its own state.

## download_whisper_model

```rust
#[tauri::command] pub fn download_whisper_model(id: String, app: AppHandle)
```

Download one model on a background thread. Emits `asr-download-progress` while it runs, then exactly one of `asr-download-done` (the id) or `asr-download-error` (a message). Returns immediately; the frontend awaits the events, never this promise - the same shape `preprocess_project` uses.

An unknown id is answered with `asr-download-error` rather than a rejected promise, so the panel has exactly one error channel to listen on.

A second request for an id already downloading is a silent no-op, NOT an error: the first download's events are still arriving, so the panel stays correct without being told anything. Telling it "already downloading" would be an error message for something the user did not do wrong.

The `cancel` closure passed to `download_model` is `|| false` for now - no cancel command exists yet. An interrupted app simply leaves the `.part` on disk and the next attempt resumes from it, which is the same outcome a cancel would produce.

## Progress

```rust
pub struct Progress { pub phase: String, pub pct: u32 }
```

The `asr-progress` payload. `phase` is `"decode"` while ffmpeg reads the WAV (brief) and then `"transcribe"` for the whisper run (where all the time goes), each counting 0 to 100 of its own. Two phases rather than one blended percentage because they are wildly different lengths: a single bar would sit at 2% for a second and then crawl, which reads as a stall.

## transcribe_project

```rust
#[tauri::command] pub fn transcribe_project(folder: String, app: AppHandle)
```

Transcribe a project's own audio and write the result into its `edit.json`. Returns at once; the frontend awaits `asr-progress` and then exactly one of `asr-done` (the caption count, a `u32`) or `asr-error` (a message to show verbatim). The same event shape `download_whisper_model` and `preprocess_project` use.

The pass, in order: resolve the style from `edit::seed::load_or_seed(&paths).settings.captions` and the model from `models::resolve_model`; refuse with a naming message when the model file is not downloaded yet; pick the source track (`audio::pick_source`, or "This recording has no audio to transcribe."); decode it (`audio::decode_16k_mono`); run whisper (`whisper::transcribe`); tokens to words (`words::words_from_tokens`); words onto the output clock (`audio::shift_words` with `audio::source_shift_ms`); words to caption lines (`group::group_words`); then `EditOp::SetCaptions` through `edit::commands::apply_edit_op`.

**Why the write goes through `apply_edit_op` (plan ADDED-8):** that command is where the doc lock's full dance already lives - precompute the seed inputs unlocked, take `edit::lock::doc_lock` ONCE, then seed-or-load and apply and save under it. Re-implementing that here would be a second copy of a lock protocol whose whole point is that there is one. As a result a transcription is one undoable step that cannot race any other writer, and no caption array ever crosses IPC in the other direction - the frontend re-fetches with `getEdit`.

**The in-flight set doubles as the cancel signal.** `TRANSCRIBING` holds the folders with a decode running. `start` takes the slot (a second request for the same folder is a silent no-op, not an error - same reasoning as the download above), `finish` releases it on every exit path including the error one, and the abort closure handed to `whisper::transcribe` is `|| !running(folder)` - so a future Cancel button is a command that removes the folder from the set, and `whisper.cpp` stops on its next step. Nothing else has to change for it.

### Used by

- `src/shared/ipc.ts` (`whisperModels`, `downloadWhisperModel`, `transcribeProject`) - the TypeScript bindings.
- `src-tauri/src/lib.rs` - all three are registered in `generate_handler!`.

**Voice activity detection (2026-09-15).** Before decoding the audio, `run` fetches `models::VAD_MODEL` through `download_model` (a one-time 885 KB download that resumes and verifies like the speech models, its progress not surfaced) and passes the path as `AsrParams.vad_model`; a failure there is swallowed, so an offline machine still transcribes, only without whisper.cpp's Silero pass in front of the decoder. With the VAD, silence is never decoded, which is what stops base.en's hallucinated "you" on quiet stretches; `whisper::drop_silent` remains as the fallback gate.
