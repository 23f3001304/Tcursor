# src-tauri/src/ai/commands.rs

Tauri IPC commands for the AI director: `list_ollama_models` (thin passthrough for the Engine picker), `ai_autoedit` (one-shot: run the pipeline and apply the whole plan), and `ai_plan` (agentic: return the plan as ordered labeled steps WITHOUT applying, so the editor reveals them one-by-one). Both commands share the private `build_plan` LLM pass - load session artifacts, build a timeline transcript, call a local Ollama chat model, and parse the reply into safe edit operations. This is the only file in the `ai` module that touches the filesystem or the Tauri command bus; all other `ai::*` modules are pure functions.

## list_ollama_models

```rust
#[tauri::command]
pub fn list_ollama_models() -> Vec<String>
```

Tauri IPC command. Returns the names of locally-installed Ollama **chat** models (embedding-only models like `nomic-embed-text` are filtered out in `ollama::list_models`), for `AiPanel.tsx`'s Engine picker and as `build_plan`'s default-model source.

### Implementation

Delegates entirely to `ai::backend::ollama::list_models()`. No error variant: an unreachable Ollama or unparseable response both resolve to an empty `Vec`, since this is a convenience for populating a dropdown, not a precondition for `ai_autoedit` (which still works with its own default model name).

### Used by

- `src/editor/panels/AiPanel.tsx` - fetched once on mount to populate the Engine `Picker`; falls back to showing just the currently-selected (or default) model name when the list is empty.

## ai_autoedit

```rust
#[tauri::command]
pub fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String>
```

Tauri IPC command. Runs the full pipeline for the session at `folder`, applies the whole plan, and returns the updated edit document (the non-agentic, one-shot path).

### Inputs

- `folder: String` - absolute path to the project session directory. *Why `String`:* Tauri IPC round-trips `String` cleanly; `PathBuf` would need a custom deserializer.
- `model: Option<String>` - Ollama chat model to use. When `None`, empty, or a model that isn't installed, `build_plan` falls back to the first installed chat model; there is **no** hardcoded default (a missing model 404s, which was the "AI returned 404" bug).

### Implementation

1. `build_plan(&paths, model)` (below) runs the whole LLM pass and returns `(doc, ops, log, dur_ms)`.
2. Replace rather than merge: clear `doc.zooms`, reset `doc.trim` to `Trim { in_ms: 0, out_ms: dur_ms }` (full clip), then apply each `EditOp` via `edit::ops::api::apply`. *Why clear-then-apply:* the mechanical seed-time auto-zooms are replaced entirely by the AI plan, not overlaid.
3. Save `doc` to `paths.edit()` and return it. On any error before the save, `edit.json` is left untouched.

## ai_plan

```rust
#[tauri::command]
pub fn ai_plan(folder: String, model: Option<String>) -> Result<Vec<AiStep>, String>
```

Tauri IPC command. Same LLM pass as `ai_autoedit`, but returns the plan as ordered, labeled `AiStep`s **without applying anything**. The editor applies them one-at-a-time (each via `apply_edit_op`) so auto-edit reads like a live agent editing the panels. When the doc already has zooms, the first step is `EditOp::ClearZooms` (labeled `"Rethinking your zooms…"`) so the reveal shows the mechanical zooms give way to the smart ones; each remaining step's `label` comes from `ai::backend::narrate::label_for`.

### Used by

- `src/editor/Editor.tsx` - `onRun` awaits `aiPlan`, then applies + narrates each step ~460ms apart, scrubbing the preview to each zoom. One `record(doc)` before the loop makes the whole pass a single undo.

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

The shared LLM pass behind both commands: load-or-seed the doc; compute the TRUE clip length via `edit::seed::true_duration_ms` (NOT `doc.trim.out_ms`, which is 0 after a trim reset and would feed the AI a zero-length timeline); load the event/action/cursor/typing logs; serialize a transcript (`ai::backend::timeline::serialize`); pick an installed chat model (caller's pick if present locally, else the first from `ollama::list_models()`, else an error); call `ollama::chat`; and parse with `plan::ops_from_json`. Returns `(doc, ops, event log, clip length ms)` - the event log is threaded out so `ai_plan` can narrate each zoom against the click that triggered it. `?`-propagates every failure so a bad LLM pass never mutates `edit.json`.
