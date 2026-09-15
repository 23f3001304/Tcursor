# src-tauri/src/ai/commands.rs

Tauri IPC surface for the AI director. `ai_propose` runs the whole propose pass and returns proposals, applying nothing; `list_ollama_models` populates the Engine picker; `pick_model` resolves the user's choice against what is actually pulled. The work itself lives in `ai::run`; this file is the boundary.

**`ai_autoedit` removed (bug-sweep-2 Task 7g).** A one-shot command that ran the LLM pass then blind-saved the WHOLE doc onto a pre-network-call snapshot (M2: any `apply_edit_op`/`save_edit` the user issued during the up-to-180s Ollama call was silently discarded when the stale snapshot landed). Verified to have zero frontend callers. The review sheet is the shape that replaces it: the frontend applies the accepted proposals through `apply_edit_op`, which re-reads `edit.json` per op, so the staleness hazard cannot come back.

**Off the main thread (Task 40).** Every command here is `async fn` wrapping its blocking work in `tauri::async_runtime::spawn_blocking`. Tauri v2 dispatches a non-`async` `#[tauri::command] fn` INLINE on the thread that received the IPC message (see `tauri-macros::command::wrapper::body_blocking` - no pool, no `spawn`), which for the webview's IPC is the app's main/UI thread. The Ollama client is synchronous by design, a first-run model load can take minutes, and a propose pass also spawns ffmpeg per frame - a sync version would leave the window OS-unresponsive for the whole run.

## list_ollama_models

```rust
#[tauri::command]
pub async fn list_ollama_models() -> Vec<OllamaModel>
```

Locally-installed Ollama CHAT models, each paired with whether it can read images, for `AiPanel.tsx`'s Engine picker and its "Vision" badge.

### Implementation

`spawn_blocking` over `ollama::list_models` plus `vision::has_vision` per name, `.unwrap_or_default()`. No error variant: an unreachable Ollama, an unparseable response, or a join failure all resolve to an empty `Vec`, because this populates a dropdown rather than gating a run. `has_vision` caches per model, so the repeat calls a remount makes are free.

### Used by

- `src/editor/panels/AiPanel.tsx` - fetched on mount to populate the Engine `Picker`; an empty list renders the honest "None found" state with a Retry, not a fabricated default.

## pick_model

```rust
pub(crate) fn pick_model(requested: Option<String>, installed: &[String]) -> Result<String, String>
```

Pure (no I/O): `requested` if it is non-empty AND present in `installed`, else the first entry of `installed`, else an error naming the `ollama pull` that fixes it. Never a hardcoded name - Ollama 404s on a model that is not pulled, which was the "AI returned 404" bug. Extracted so the selection is unit-testable without a live Ollama.

## ai_propose

```rust
#[tauri::command]
pub async fn ai_propose(folder: String, model: Option<String>) -> Result<AiRun, String>
```

Look at the recording and return a reviewable list of proposed edits, applying NOTHING.

### Inputs

- `folder` - absolute path to the project session directory. *Why `String`:* Tauri IPC round-trips it cleanly; `PathBuf` would need a custom deserializer.
- `model` - the Ollama chat model to use. `None`, empty, or not installed falls back through `pick_model`.

### Returns

`Ok(AiRun)` including when nothing was worth proposing: an empty `proposals` is an answer the sheet states in words. `Err(String)` with a message the user can act on, shown with a Retry.

### Implementation

`spawn_blocking` over `ai::run::propose`. Everything else is in `ai/run.md`.

## tests

`#[cfg(test)] mod tests` covers `pick_model`: keeps the caller's choice when it is installed; falls back to the first installed model when the choice is missing, empty, or not pulled; errors when nothing is installed at all.
