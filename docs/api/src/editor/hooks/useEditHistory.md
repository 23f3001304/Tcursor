# src/editor/hooks/useEditHistory.ts

Frontend undo/redo for the `EditDoc`. There's no backend history - the doc IS the whole edit state, so undo/redo just swap between two snapshot stacks, persist the restored doc, and bump `rev` so the preview refetches. `undo`/`redo` are routed through a caller-supplied `enqueue` (see `opQueue.ts`) so they can never race an in-flight `applyOp`.

## useEditHistory

```ts
export function useEditHistory(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  bumpRev: () => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
  onSwap?: (kind: "undo" | "redo") => void,
): { record: (current: EditDoc) => void; undo: () => Promise<void>; redo: () => Promise<void>; canUndo: boolean; canRedo: boolean }
```

### Inputs

- `folder: string` - project directory, needed to persist the restored doc via `saveEdit` on undo/redo.
- `docRef: RefObject<EditDoc | null>` - `Editor`'s `docRef` (mirrors its `doc` state, reassigned every render). `swap` (used by both `undo` and `redo`) reads `docRef.current` - NOT a plain `doc` value param - at the moment it actually RUNS, and pushes that onto the opposite stack before replacing it. *Why a ref, not a value:* `swap` only ever executes from inside `enqueue`, which can delay it well past the render that triggered the undo/redo (e.g. behind an in-flight `applyOp`); a plain captured `doc` value would still reflect whatever was current when `undo`/`redo` was CALLED, not whatever the doc actually settled to by the time `swap` runs - the exact same staleness `applyOp` guards against with its own `docRef` read (see `Editor.md`).
- `setDoc: (d: EditDoc) => void` - swaps in the restored doc.
- `bumpRev: () => void` - called after a successful undo/redo so the camera curve and other `rev`-keyed preview data refetch.
- `enqueue: <T>(fn: () => Promise<T>) => Promise<T>` - the SAME queue `Editor` hands to `applyOp` (one `createQueue()` instance per editor session). Both `undo` and `redo` wrap their actual swap in this, so an undo/redo issued while an `applyOp` IPC round trip is still pending waits for that apply to settle first - without this, a fast `Ctrl+Z` mid-apply could `setDoc` the restored snapshot only for the in-flight apply's OWN `setDoc` to land after and silently overwrite it.
- `onSwap?: (kind: "undo" | "redo") => void` - optional, called once a swap actually happens (i.e. the source stack was non-empty) with which direction just ran. `Editor` wires this to `useUndoToast`'s `onSwap` to fire the "Undid"/"Redid" pill. Never called for a no-op undo/redo (empty stack, or `docRef.current` is `null`).

### Returns

- `record: (current: EditDoc) => void` - call BEFORE applying an edit, with the CURRENT (pre-edit) doc. Pushes `current` onto the undo stack, unless the previous push was less than `COALESCE_MS` (400ms) ago - a rapid burst (e.g. a slider drag firing many ops) then coalesces into a single pre-drag snapshot, so one undo reverts the whole drag rather than one tick of it. Always clears the redo stack (a fresh edit invalidates whatever was undone before it) and caps the undo stack at `LIMIT` (100 steps), dropping the oldest. Callers pass `docRef.current` (read fresh at their own execution time), not a captured `doc` variable - see `Editor.md`'s `applyOp`/`onRun`.
- `undo: () => Promise<void>` / `redo: () => Promise<void>` - `enqueue` a swap: pop the top of the undo/redo stack, push the doc being left onto the OTHER stack (`docRef.current`, read at execution time), `setDoc` the popped doc, persist it via `saveEdit` (swallowing a write failure so the UI still reflects the swap even if the disk write fails), `bumpRev()`, then call `onSwap` with the direction. No-op (and no `onSwap` call) if the source stack is empty or `docRef.current` is `null`. Resets the coalescing timer so the next edit after an undo/redo always starts its own step rather than merging into whatever preceded the undo.
- `canUndo` / `canRedo: boolean` - whether the respective stack is non-empty; re-synced after every `record`/`undo`/`redo`.

### Implementation

- The stacks (`undoStack`, `redoStack`) and the coalescing timestamp (`lastAt`) live in refs, not state - they don't need to trigger renders themselves; `canUndo`/`canRedo` are the only pieces of derived state exposed for the UI to react to (e.g. disabling `TopBar`'s undo/redo buttons).
- A global `keydown` listener maps Ctrl/Cmd+Z to `undo`, and Ctrl/Cmd+Shift+Z or Ctrl+Y to `redo`. Ignored while the event target is an `<input>`, `<textarea>`, or `contentEditable` element, so a text field's own native undo still works. Because this listener calls the SAME `undo`/`redo` functions returned to the caller, keyboard-triggered undo/redo goes through `enqueue` (and fires `onSwap`) exactly like a button click does - there is no separate, unserialized path.
- `swap`'s `useCallback` deps are `[docRef, folder, setDoc, bumpRev, onSwap]` - `docRef` itself (the ref OBJECT) is referentially stable for the whole editor session, so unlike the old `doc`-value dependency, `swap` (and therefore `undo`/`redo`) no longer needs to be recreated on every doc mutation. It reads `docRef.current` fresh on each actual call instead.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `record(docRef.current)` is called before every `applyOp` mutation and once before the AI director's whole `onRun` pass (so the agentic reveal undoes as one step); `saveDocSettings` (`useDocSettings.ts`) still records against a plain `doc` value, not `docRef` - it isn't routed through `enqueue` (out of scope for the queue's serialization fix; a settings write races nothing since it isn't deferred), so it has no execution-time staleness to guard against. `undo`/`redo`/`canUndo`/`canRedo` are wired to `TopBar`'s undo/redo buttons; `enqueue` is `Editor`'s own `createQueue()` instance (shared with `applyOp` and `onRun`) and `onSwap` is `useUndoToast`'s `onSwap` (`src/editor/shell/Toast.tsx`).
