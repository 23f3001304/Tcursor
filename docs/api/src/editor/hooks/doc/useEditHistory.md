# src/editor/hooks/doc/useEditHistory.ts

Frontend undo/redo for the `EditDoc`. There's no backend history - the doc IS the whole edit state, so undo/redo just swap between two snapshot stacks, persist the restored doc, and bump `rev` so the preview refetches. `undo`/`redo` are routed through a caller-supplied `enqueue` (see `opQueue.ts`) so they can never race an in-flight `applyOp`.

## shouldPushNewSnapshot

```ts
export function shouldPushNewSnapshot(stackLen: number, lastAt: number, now: number, coalesceMs: number = COALESCE_MS): boolean
```

Pure: should `record` push a NEW undo snapshot, or coalesce into the top of an already-pending one? `true` when the undo stack is empty, or the gap since the last push exceeds `coalesceMs` (400ms default). Extracted (bug-sweep-2 Task 8 review round 1) so the coalescing window's exact edges are unit-tested (`useEditHistory.test.ts`) without mounting the hook.

## matchesStackTop

```ts
export function matchesStackTop<T>(stack: T[], snapshot: T | null): boolean
```

Pure: is `snapshot` still exactly the top of `stack` (reference equality)? `false` for a `null` snapshot or an empty stack. This is the identity guard `unrecord` uses (review round 1, Important 4) so a stale or duplicate unrecord call can never pop a DIFFERENT, real edit's snapshot - only the exact one it pushed, and only while nothing has pushed on top of it since. `EditDoc` objects are never mutated in place anywhere in this codebase (every mutation produces a fresh object via `setDoc`), so reference equality is a sound "is this literally the same push" test.

## RecordToken

```ts
export interface RecordToken { snapshot: EditDoc | null; prevLastAt: number; prevRedo: EditDoc[] }
```

Everything `record` hands back for `unrecord` to fully roll itself back with (review round 1, Important 4 - replaces a plain `boolean`): `snapshot` is the doc `record` actually pushed (`null` when coalesced - nothing to roll back), and `prevLastAt`/`prevRedo` are the PRE-CALL coalescing timestamp and redo stack, captured so `unrecord` can restore them verbatim. Without capturing these, undoing a failed edit's bookkeeping would leave the coalescing window "poisoned" (the NEXT genuine edit could wrongly merge into an unrelated, older undo step) or silently discard a real redo trail that `record` cleared for an edit that never actually happened.

## useEditHistory

```ts
export function useEditHistory(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  bumpRev: () => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
  onSwap?: (kind: "undo" | "redo") => void,
): { record: (current: EditDoc) => RecordToken; unrecord: (token: RecordToken) => void; undo: () => Promise<void>; redo: () => Promise<void>; canUndo: boolean; canRedo: boolean }
```

### Inputs

- `folder: string` - project directory, needed to persist the restored doc via `saveEdit` on undo/redo.
- `docRef: RefObject<EditDoc | null>` - `Editor`'s `docRef` (mirrors its `doc` state, reassigned every render). `swap` (used by both `undo` and `redo`) reads `docRef.current` - NOT a plain `doc` value param - at the moment it actually RUNS, and pushes that onto the opposite stack before replacing it. *Why a ref, not a value:* `swap` only ever executes from inside `enqueue`, which can delay it well past the render that triggered the undo/redo (e.g. behind an in-flight `applyOp`); a plain captured `doc` value would still reflect whatever was current when `undo`/`redo` was CALLED, not whatever the doc actually settled to by the time `swap` runs - the exact same staleness `applyOp` guards against with its own `docRef` read (see `Editor.md`).
- `setDoc: (d: EditDoc) => void` - swaps in the restored doc.
- `bumpRev: () => void` - called after a successful undo/redo so the camera curve and other `rev`-keyed preview data refetch.
- `enqueue: <T>(fn: () => Promise<T>) => Promise<T>` - the SAME queue `Editor` hands to `applyOp` (one `createQueue()` instance per editor session). Both `undo` and `redo` wrap their actual swap in this, so an undo/redo issued while an `applyOp` IPC round trip is still pending waits for that apply to settle first - without this, a fast `Ctrl+Z` mid-apply could `setDoc` the restored snapshot only for the in-flight apply's OWN `setDoc` to land after and silently overwrite it.
- `onSwap?: (kind: "undo" | "redo") => void` - optional, called once a swap actually happens (i.e. the source stack was non-empty) with which direction just ran. `Editor` wires this to `useUndoToast`'s `onSwap` to fire the "Undid"/"Redid" pill. Never called for a no-op undo/redo (empty stack, or `docRef.current` is `null`).

### Returns

- `record: (current: EditDoc) => RecordToken` - call BEFORE applying an edit, with the CURRENT (pre-edit) doc. Pushes `current` onto the undo stack per `shouldPushNewSnapshot` (a rapid burst - e.g. a slider drag firing many ops - coalesces into a single pre-drag snapshot, so one undo reverts the whole drag rather than one tick of it), caps the undo stack at `LIMIT` (100 steps, dropping the oldest), and always clears the redo stack (a fresh edit invalidates whatever was undone before it - restorable via the returned token if the edit turns out not to have happened). Callers pass `docRef.current` (read fresh at their own execution time), not a captured `doc` variable - see `Editor.md`'s `applyOp`/`onRun` and `useDocSettings.md`'s `write`. Returns a `RecordToken` (see above) - `unrecord` needs the WHOLE token, not just a boolean, to fully undo this call's bookkeeping.
- `unrecord: (token: RecordToken) => void` - rolls back exactly the `record()` call that produced `token` (bug-sweep-2 Task 8, L1; hardened in review round 1, Important 4). Identity-checked via `matchesStackTop(undoStack, token.snapshot)` - a NO-OP if `token.snapshot` is `null` (the `record` it came from coalesced - nothing to undo) or if something else has pushed onto the undo stack since (the token is stale; popping would remove a DIFFERENT, real edit's entry). When it DOES match, pops the snapshot, restores `lastAt`/the redo stack to `token.prevLastAt`/`token.prevRedo`. `Editor.tsx`'s `applyOp` calls this in its `catch`, so a rejected `apply_edit_op` never leaves a phantom undo step (the Undo button lighting up over nothing, and the first Ctrl+Z being a silent no-op with a false "Undid" toast) - and, as of the token hardening, never poisons the coalescing window or drops a real redo trail either.
- `undo: () => Promise<void>` / `redo: () => Promise<void>` - `enqueue` a swap: pop the top of the undo/redo stack, push the doc being left onto the OTHER stack (`docRef.current`, read at execution time), `setDoc` the popped doc AND write it back to `docRef.current` (see the M1 section below), persist it via `saveEdit` (swallowing a write failure so the UI still reflects the swap even if the disk write fails), `bumpRev()`, then call `onSwap` with the direction. No-op (and no `onSwap` call) if the source stack is empty or `docRef.current` is `null`. Resets the coalescing timer so the next edit after an undo/redo always starts its own step rather than merging into whatever preceded the undo.
- `canUndo` / `canRedo: boolean` - whether the respective stack is non-empty; re-synced after every `record`/`unrecord`/`undo`/`redo`.

### Implementation

- The stacks (`undoStack`, `redoStack`) and the coalescing timestamp (`lastAt`) live in refs, not state - they don't need to trigger renders themselves; `canUndo`/`canRedo` are the only pieces of derived state exposed for the UI to react to (e.g. disabling `TopBar`'s undo/redo buttons).
- A global `keydown` listener maps Ctrl/Cmd+Z to `undo`, and Ctrl/Cmd+Shift+Z or Ctrl+Y to `redo`. Ignored while the event target is an `<input>`, `<textarea>`, or `contentEditable` element, so a text field's own native undo still works. Because this listener calls the SAME `undo`/`redo` functions returned to the caller, keyboard-triggered undo/redo goes through `enqueue` (and fires `onSwap`) exactly like a button click does - there is no separate, unserialized path.
- `swap`'s `useCallback` deps are `[docRef, folder, setDoc, bumpRev, onSwap]` - `docRef` itself (the ref OBJECT) is referentially stable for the whole editor session, so unlike the old `doc`-value dependency, `swap` (and therefore `undo`/`redo`) no longer needs to be recreated on every doc mutation. It reads `docRef.current` fresh on each actual call instead.

### `docRef.current` writes at execution time (bug-sweep-2 Task 8, M1; review round 1, Important 2)

This hook's whole design depends on `docRef.current` being CORRECT the instant a queued task runs, not just eventually - and a queued task runs as a **microtask** the moment the ONE ahead of it in `enqueue`'s chain settles (`chain.then(fn, fn)`, `opQueue.ts`), which is BEFORE React's macrotask-scheduled re-render from a `setDoc` call ever lands (microtasks always drain first). `Editor.tsx` mirrors `doc` into `docRef` at RENDER time (`docRef.current = doc`, every render) - on its own, that is too late for this race. Every site that mutates the doc from INSIDE a queued task therefore ALSO writes `docRef.current` directly, synchronously, before its own promise resolves:

- `swap` (this file) - writes `docRef.current = next` right after `setDoc(next)`.
- `Editor.tsx`'s `applyOp` - writes `docRef.current = d` right after `setDoc(d)`.
- `useDocSettings.ts`'s `write` - writes `docRef.current = newDoc` right after `setDoc(newDoc)` (review round 1, Critical 1 - see `useDocSettings.md`).
- `applyOps.ts`'s `applyRun` - writes `docRef.current = next` right after `setDoc(next)` for each applied op of an AI review sheet (review round 1, Important 2 - see `applyOps.md`).

Without this at any ONE of these sites, a queued task immediately behind it (an undo/redo, another apply, a settings write) would read a STALE `docRef.current` - `swap` specifically would push the stale doc onto the counterpart stack, making whatever the SKIPPED site actually landed become unrecoverable by either undo or redo (the doc "IS the whole edit state" invariant this whole hook depends on). The adversarial "two rapid Ctrl+Z, fast-resolving `saveEdit`" scenario (`useEditHistory.test.ts`) pins exactly this for `swap`: without the write, the second queued undo's redo-stack push duplicates the first's instead of capturing the doc it actually left.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `record(docRef.current)` is called before every `applyOp` mutation and once before the AI director's whole `onRun` pass (so the agentic reveal undoes as one step), with `unrecord(token)` popping that snapshot back off in `applyOp`'s `catch` when the apply it guarded fails; `saveDocSettings` (`useDocSettings.ts`) also calls `record` (now at EXECUTION time, inside its own enqueued task - see `useDocSettings.md`) and is routed through the SAME `enqueue` queue as `applyOp`/`undo`/`redo` (bug-sweep-2 Task 8, M5). `undo`/`redo`/`canUndo`/`canRedo` are wired to `TopBar`'s undo/redo buttons; `enqueue` is `Editor`'s own `createQueue()` instance (shared with `applyOp`, `onRun`, and `useDocSettings.write`) and `onSwap` is `useUndoToast`'s `onSwap` (`src/editor/shell/dialogs/Toast.tsx`).
