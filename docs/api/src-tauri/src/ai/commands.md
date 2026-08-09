# src-tauri/src/ai/commands.rs

Tauri IPC commands for the AI director: `list_ollama_models` (thin passthrough for the Engine picker), `ai_autoedit` (one-shot: run the pipeline and apply the whole plan), and `ai_plan` (agentic: return the plan as ordered labeled steps WITHOUT applying, so the editor reveals them one-by-one). Both commands share the private `build_plan` LLM pass - load session artifacts, build a timeline transcript, call a local Ollama chat model, and parse the reply into safe edit operations. This is the only file in the `ai` module that touches the filesystem or the Tauri command bus; all other `ai::*` modules are pure functions.

**Off the main thread (Task 40).** All three commands are `async fn`; each wraps its blocking work in `tauri::async_runtime::spawn_blocking`. This matters because Tauri v2 dispatches a non-`async` `#[tauri::command] fn` INLINE on the thread that received the IPC message (see `tauri-macros::command::wrapper::body_blocking` - no pool, no `spawn`), which for the webview's IPC is the app's main/UI thread. `ollama.rs`'s HTTP client (`ureq`) is synchronous by design (no async runtime dependency added), and a first-run Ollama model load can take minutes - a sync version of these commands would leave the entire window OS-unresponsive for the duration of every AI Director run. `spawn_blocking` moves that blocking work onto Tokio's blocking-thread pool while the command itself `.await`s it, so the main thread stays free to pump the event loop.

## list_ollama_models

```rust
#[tauri::command]
pub async fn list_ollama_models() -> Vec<String>
```

Tauri IPC command. Returns the names of locally-installed Ollama **chat** models (embedding-only models like `nomic-embed-text` are filtered out in `ollama::list_models`), for `AiPanel.tsx`'s Engine picker and as `build_plan`'s default-model source.

### Implementation

`spawn_blocking(ollama::list_models)`, `.await`ed, `.unwrap_or_default()`. No error variant: an unreachable Ollama, an unparseable response, or a `spawn_blocking` join failure all resolve to an empty `Vec`, since this is a convenience for populating a dropdown, not a precondition for `ai_autoedit` (which still works with its own default model name).

### Used by

- `src/editor/panels/AiPanel.tsx` - fetched once on mount to populate the Engine `Picker`; falls back to showing just the currently-selected (or default) model name when the list is empty.

## pick_model

```rust
fn pick_model(requested: Option<String>, installed: &[String]) -> Result<String, String>
```

Pure (no I/O) helper `build_plan` uses to pick the chat model it sends to Ollama: `requested` if it's non-empty AND present in `installed`, else the first entry of `installed`, else an error (`"No Ollama models are installed..."`). Never returns a hardcoded name - Ollama 404s on a model that isn't pulled (the bug this guards against). Extracted specifically so this selection logic is unit-testable without a live Ollama (see `tests` below).

## ai_autoedit

```rust
#[tauri::command]
pub async fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String>
```

Tauri IPC command. Runs the full pipeline for the session at `folder`, applies the whole plan, and returns the updated edit document (the non-agentic, one-shot path).

### Inputs

- `folder: String` - absolute path to the project session directory. *Why `String`:* Tauri IPC round-trips `String` cleanly; `PathBuf` would need a custom deserializer.
- `model: Option<String>` - Ollama chat model to use. When `None`, empty, or a model that isn't installed, `pick_model` (via `build_plan`) falls back to the first installed chat model; there is **no** hardcoded default (a missing model 404s, which was the "AI returned 404" bug).

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking(move || { ... })`, `.await`ed then `?`-unwrapped (a join failure maps to `Err(String)`, same shape as any other failure this command can return):

1. `build_plan(&paths, model)` (below) runs the whole LLM pass and returns `(doc, ops, log, dur_ms)`.
2. Replace rather than merge: clear `doc.zooms`, reset `doc.trim` to `Trim { in_ms: 0, out_ms: dur_ms }` (full clip), then apply each `EditOp` via `edit::ops::api::apply`. *Why clear-then-apply:* the mechanical seed-time auto-zooms are replaced entirely by the AI plan, not overlaid.
3. Save `doc` to `paths.edit()` and return it. On any error before the save, `edit.json` is left untouched.

## ai_plan

```rust
#[tauri::command]
pub async fn ai_plan(folder: String, model: Option<String>) -> Result<Vec<AiStep>, String>
```

Tauri IPC command. Same LLM pass as `ai_autoedit`, but returns the plan as ordered, labeled `AiStep`s **without applying anything**. The editor applies them one-at-a-time (each via `apply_edit_op`) so auto-edit reads like a live agent editing the panels. When the doc already has zooms, the first step is `EditOp::ClearZooms` (labeled `"Rethinking your zooms…"`) so the reveal shows the mechanical zooms give way to the smart ones; each remaining step's `label` comes from `ai::backend::narrate::label_for`. Body runs inside `spawn_blocking` the same way as `ai_autoedit` above.

### Used by

- `src/editor/Editor.tsx` (via `src/editor/director/useDirector.ts`) - `useDirector`'s `run` fetches the plan through `planOrCancel(cancelRef, () => aiPlan(...))` (Task 40 - makes the fetch window user-cancellable even though the underlying HTTP call itself always runs to completion), then applies + narrates each step, scrubbing the preview to each zoom. One `record(doc)` before the fetch makes the whole pass a single undo.

## AiStep

```rust
#[derive(serde::Serialize)]
pub struct AiStep { pub op: EditOp, pub label: String }
```

One step of the plan: the `EditOp` to apply plus a human "what I did + why" line for the agentic reveal log.

## build_plan

```rust
fn build_plan(paths: &ProjectPaths, model: Option<String>) -> Result<(EditDoc, Vec<EditOp>, EventLog, u32), String>
```

The shared LLM pass behind both commands - BLOCKING (file I/O + `ollama::chat`'s network call): load-or-seed the doc; compute the TRUE clip length via `edit::seed::true_duration_ms` (NOT `doc.trim.out_ms`, which is 0 after a trim reset and would feed the AI a zero-length timeline); load the event/action/cursor/typing logs; compute `shift = edit::seed::output_shift(paths)` (the SAME recipe `edit::migrate::v1_to_v2` uses to put seeded regions on the output clock - reused, not re-derived); serialize a transcript on that clock (`ai::backend::timeline::serialize(..., dur_ms, shift)`); pick an installed chat model via `pick_model` (above); call `ollama::chat`; and parse with `plan::ops_from_json`. Returns `(doc, ops, event log, clip length ms)` - the event log is threaded out so `ai_plan` can narrate each zoom against the click that triggered it. `?`-propagates every failure so a bad LLM pass never mutates `edit.json`. Callers MUST run this inside `spawn_blocking` (both commands above do) - it is never awaited directly on a command's async task.

*Why the transcript needs shifting:* `ops_from_json`'s `AddZoomFull { at_ms }` ops are applied straight onto `doc.zooms`, which is OUTPUT time (`edit::model::EditDoc`'s one-clock contract) - so without this shift, every AI-placed zoom would land `events_ms - video_start` (≈800 ms) away from the click the model actually reasoned about.

## tests

`#[cfg(test)] mod tests` covers `pick_model`: keeps the caller's choice when it's installed; falls back to the first installed model when the choice is missing, empty, or not pulled; errors when nothing is installed at all.
