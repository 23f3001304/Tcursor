# src/editor/hooks/useEditHistory.ts

Frontend undo/redo for the `EditDoc`. There's no backend history - the doc IS the whole edit state, so undo/redo just swap between two snapshot stacks, persist the restored doc, and bump `rev` so the preview refetches.

## useEditHistory

```ts
export function useEditHistory(
  folder: string,
  doc: EditDoc | null,
  setDoc: (d: EditDoc) => void,
  bumpRev: () => void,
): { record: (current: EditDoc) => void; undo: () => Promise<void>; redo: () => Promise<void>; canUndo: boolean; canRedo: boolean }
```

### Inputs

- `folder: string` - project directory, needed to persist the restored doc via `saveEdit` on undo/redo.
- `doc: EditDoc | null` - the current doc; `swap` (used by both `undo` and `redo`) pushes this onto the opposite stack before replacing it, so a later undo/redo can get back to it.
- `setDoc: (d: EditDoc) => void` - swaps in the restored doc.
- `bumpRev: () => void` - called after a successful undo/redo so the camera curve and other `rev`-keyed preview data refetch.

### Returns

- `record: (current: EditDoc) => void` - call BEFORE applying an edit, with the CURRENT (pre-edit) doc. Pushes `current` onto the undo stack, unless the previous push was less than `COALESCE_MS` (400ms) ago - a rapid burst (e.g. a slider drag firing many ops) then coalesces into a single pre-drag snapshot, so one undo reverts the whole drag rather than one tick of it. Always clears the redo stack (a fresh edit invalidates whatever was undone before it) and caps the undo stack at `LIMIT` (100 steps), dropping the oldest.
- `undo: () => Promise<void>` / `redo: () => Promise<void>` - pop the top of the undo/redo stack, push the doc being left onto the OTHER stack, `setDoc` the popped doc, persist it via `saveEdit` (swallowing a write failure so the UI still reflects the swap even if the disk write fails), then `bumpRev()`. No-op if the source stack is empty or `doc` is `null`. Resets the coalescing timer so the next edit after an undo/redo always starts its own step rather than merging into whatever preceded the undo.
- `canUndo` / `canRedo: boolean` - whether the respective stack is non-empty; re-synced after every `record`/`undo`/`redo`.

### Implementation

- The stacks (`undoStack`, `redoStack`) and the coalescing timestamp (`lastAt`) live in refs, not state - they don't need to trigger renders themselves; `canUndo`/`canRedo` are the only pieces of derived state exposed for the UI to react to (e.g. disabling `TopBar`'s undo/redo buttons).
- A global `keydown` listener maps Ctrl/Cmd+Z to `undo`, and Ctrl/Cmd+Shift+Z or Ctrl+Y to `redo`. Ignored while the event target is an `<input>`, `<textarea>`, or `contentEditable` element, so a text field's own native undo still works.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `record(doc)` is called before every `applyOp`/`saveDocSettings` mutation (and once before the AI director's whole `onRun` pass, so the agentic reveal undoes as one step); `undo`/`redo`/`canUndo`/`canRedo` are wired to `TopBar`'s undo/redo buttons.
