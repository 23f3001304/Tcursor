# src-tauri/src/edit/commands.rs

Tauri IPC command handlers that bridge the frontend editor to the `edit` layer. Three commands cover the full read-mutate-save lifecycle; the frontend never touches `edit.json` directly.

## get_edit

```rust
#[tauri::command]
pub fn get_edit(folder: String) -> Result<EditDoc, String>
```

Loads (or seeds) the `EditDoc` for a project folder and returns it to the frontend.

### Inputs

- `folder: String` - absolute path to the project directory. *Why String rather than PathBuf:* Tauri IPC deserializes command arguments from JSON, where paths are strings.

### Returns

`Ok(EditDoc)` on success; `Err(String)` on an unexpected I/O or parse failure (rare - `load_or_seed` silently seeds a default on missing/corrupt files).

### Implementation

1. Construct `ProjectPaths` from `folder`.
2. Delegate to `edit::seed::load_or_seed`, which returns an existing `edit.json` or builds and writes a default one from the recording.
3. Return the doc.

## apply_edit_op

```rust
#[tauri::command]
pub fn apply_edit_op(folder: String, op: EditOp) -> Result<EditDoc, String>
```

Applies one `EditOp` to the project's `EditDoc`, persists the result, and returns the updated doc.

### Inputs

- `folder: String` - project directory. *Why passed on every call:* Tauri commands are stateless; the folder identifies which project to mutate.
- `op: EditOp` - the edit operation to apply. *Why the full enum rather than a free-form JSON blob:* the enum gives serde a compile-time schema, so invalid ops are rejected at deserialization rather than silently ignored mid-mutation.

### Returns

`Ok(EditDoc)` - the post-mutation document, returned so the frontend can update its local state without a second round trip. `Err(String)` if `doc.save` fails (e.g. disk full).

### Implementation

1. Construct `ProjectPaths`.
2. Load (or seed) the current `EditDoc` via `load_or_seed`.
3. Apply `op` via `edit::api::apply`.
4. Save the mutated doc to `paths.edit()`.
5. Return the mutated doc.

## save_edit

```rust
#[tauri::command]
pub fn save_edit(folder: String, doc: EditDoc) -> Result<(), String>
```

Overwrites `edit.json` with a caller-supplied `EditDoc`. Used when the frontend holds a modified doc it has assembled locally (e.g. after a drag operation that batches many changes) rather than applying them one at a time.

### Inputs

- `folder: String` - project directory. *Why the folder rather than a full path:* `ProjectPaths` derives all file locations from a single root, keeping the IPC surface minimal.
- `doc: EditDoc` - the document to persist. *Why accept the whole doc:* allows the frontend to apply multiple local mutations atomically in one save, avoiding multiple round trips for bulk operations like AI plan application.

### Returns

`Ok(())` on success; `Err(String)` on I/O failure.

### Implementation

1. Construct `ProjectPaths`.
2. Call `doc.save(&paths.edit())`.
