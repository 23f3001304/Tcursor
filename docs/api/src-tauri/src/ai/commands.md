# src-tauri/src/ai/commands.rs

Single Tauri IPC command that orchestrates the full AI auto-edit pipeline: load session artifacts, build a timeline transcript, call a local Ollama model, parse the reply into safe edit operations, and write back an updated `EditDoc`. This is the only file in the `ai` module that touches the filesystem or the Tauri command bus; all other `ai::*` modules are pure functions.

## ai_autoedit

```rust
#[tauri::command]
pub fn ai_autoedit(folder: String, model: Option<String>) -> Result<EditDoc, String>
```

Tauri IPC command. Runs the full AI director pipeline for the session at `folder` and returns the updated edit document.

### Inputs

- `folder: String` - absolute path string to the project session directory. *Why `String` rather than `Path`:* Tauri IPC deserializes command arguments from JSON; `String` round-trips cleanly whereas `PathBuf` requires a custom deserializer.
- `model: Option<String>` - Ollama model name to use; defaults to `"llama3.2"` when `None`. *Why optional:* lets the frontend pass a user-selected model without breaking older callers that omit the field.

### Implementation

1. Construct `ProjectPaths { folder: PathBuf::from(folder) }` to get typed accessors for all session file paths.
2. Call `edit::seed::load_or_seed(&paths)` to get (or lazily create) the `EditDoc` for this session. Read `doc.trim.out_ms` as `dur_ms` - the clip's total length in milliseconds. *Why seed rather than strict load:* the AI command may run before the user has manually opened the editor, so the doc might not exist yet.
3. Load `EventLog` from `paths.events()` with `?` propagation. *Why mandatory:* mouse click positions are required for the timeline transcript; without them the AI has nothing to place zooms against.
4. Load `ActionLog` from `paths.actions()` with `.unwrap_or_default()` on failure. *Why optional:* hotkey actions enrich the transcript but a session recorded without any hotkeys is still valid input.
5. Load `CursorTrack` from `paths.cursor()`. *Why:* IBeam spans become `"text field"` lines in the transcript, helping the model identify form-filling activity.
6. Load `TypingLog` from `paths.typing()` and extract `.ms`. *Why:* typing timestamps let the model extend zooms through typing bursts rather than only through clicks.
7. Call `ai::timeline::serialize(&log, &actions, &cursor, &typing, dur_ms)` to build the plain-text transcript. *Why a separate module:* the serialization logic is testable and reusable independently of the HTTP call.
8. Resolve the model name (`model.unwrap_or_else(|| "llama3.2".into())`). Call `ai::ollama::chat(&model_name, &ai::prompt::system_prompt(), &transcript)` with `?` propagation. *Why `?` here:* if Ollama is unreachable or returns an error, the existing `edit.json` must not be modified - early return guarantees this.
9. Call `ai::plan::ops_from_json(&raw, dur_ms)` with `?` propagation. *Why `?` here too:* an unparseable or empty response must not corrupt the edit document.
10. Only after both steps 8 and 9 succeed: clear `doc.zooms`, reset `doc.trim` to `Trim { in_ms: 0, out_ms: dur_ms }` (full clip), then apply each `EditOp` via `edit::api::apply(&mut doc, op)`. *Why clear-then-apply rather than merge:* mechanical auto-zooms generated at seed time are replaced entirely by the AI plan, not overlaid, to avoid conflicting zoom regions.
11. Save `doc` to `paths.edit()` with `?` propagation, then return `Ok(doc)`.

### Returns

`Ok(EditDoc)` with all AI-directed zooms and optional trim applied, written to disk. `Err(String)` on any failure before or during step 10 (Ollama unreachable, bad JSON, no usable edits, I/O error); in all error cases the existing `edit.json` is left untouched.
