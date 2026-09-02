# src/editor/hooks/useDocSettings.ts

Settings-only doc writes, split out of `Editor` - bulk `settings` patches (background, cursor, camera, captions, audio, the AI model) as opposed to `applyOp`'s per-`EditOp` mutations.

## useDocSettings

```ts
export function useDocSettings(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  record: (current: EditDoc) => RecordToken,
  setRev: (fn: (r: number) => number) => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
): { saveDocSettings: (s: EditDoc["settings"]) => Promise<void>; onAutoModel: (v: string) => void }
```

### Inputs

- `folder: string` - project directory, passed to `saveEdit`.
- `docRef: RefObject<EditDoc | null>` - `Editor`'s `docRef` (review round 1, Critical 1 - replaces a plain `doc` value; see `write`'s own section below for why). Both returned writers patch its CURRENT `.settings` field, read at their own execution time.
- `setDoc: (d: EditDoc) => void` - swaps in the patched doc.
- `record: (current: EditDoc) => RecordToken` - `useEditHistory`'s undo-stack push (see `useEditHistory.md` for `RecordToken`); called by `saveDocSettings`, NOT by `onAutoModel`, and now from INSIDE `write`'s own enqueued task (see below), not at call time.
- `setRev: (fn: (r: number) => number) => void` - bumped after every write so `rev`-keyed preview data refetches.
- `enqueue: <T>(fn: () => Promise<T>) => Promise<T>` (bug-sweep-2 Task 8, M5) - the SAME queue `Editor` hands to `applyOp`/`undo`/`redo` (`Editor`'s one `createQueue()` instance). `write` (below) is wrapped in this - it used to bypass the queue entirely, so an in-flight `apply_edit_op` (Rust: load-mutate-save) and a settings `save_edit` (a blind full-doc overwrite of the client's snapshot) could land in either order and silently clobber each other's result on disk; a single slider drag fires ~60 `saveDocSettings` calls a second, so this was easy to hit.

### Returns

- `saveDocSettings: (s: EditDoc["settings"]) => Promise<void>` - `write(s, true)`, recording an undo step. The normal path for every settings-editing panel; a manual pick in `AiPanel`'s Engine dropdown goes through this too, via `EditorPanels`.
- `onAutoModel: (v: string) => void` - `write({...docRef.current.settings, ai_model: v}, false)` - no undo step. The only caller is `AiPanel`'s mount-time "default `ai_model` to a real installed Ollama model" effect: since that effect fires from mounting/model-list-resolving, not a user edit, recording would push a phantom undo step and an unasked-for disk write the instant the AI panel opens.

### `write` (internal)

```ts
write(nextSettings: EditDoc["settings"], recordUndo: boolean): Promise<void>
```

Both flavors funnel through this, `enqueue`d - and, as of review round 1 (Critical 1), everything it needs comes from `docRef.current` read INSIDE the enqueued task, not from a `doc` value closed over at call time:

1. Reads `const doc = docRef.current` - the doc as it stands the moment this task actually STARTS running, not whenever the caller clicked.
2. If `recordUndo`, calls `record(doc)` - AT THIS POINT, so the undo snapshot is the doc this settings change is actually about to land on, not a stale pre-queue one.
3. Builds `newDoc = {...doc, settings: nextSettings}`, `setDoc(newDoc)`, and writes `docRef.current = newDoc` immediately (same reason `applyOp`/`swap` do - see `useEditHistory.md`'s M1 section - a queued task right behind this one needs the fresh doc, not a stale ref).
4. `try`s `saveEdit(folder, newDoc)`, swallowing a rejection (`catch { /* keep the UI state even if the disk write fails */ }` - matches `useEditHistory.swap`'s own precedent) rather than letting an `unhandledrejection` fire and `setRev` never run.
5. `setRev` runs unconditionally either way.

**Why this matters (the Critical 1 bug it fixes).** Before threading `docRef` through, `write` closed over a `doc` VALUE - fine for a call that runs immediately, but `enqueue` can DELAY execution (e.g. behind an in-flight `applyOp`'s IPC round trip). A settings write issued in that window - e.g. release a timeline pill drag, then immediately toggle any setting - would build `newDoc` from the STALE pre-drag `doc`, silently dropping every non-settings field the drag's `applyOp` had just landed (its result reverted on both disk and in `setDoc`, the instant the settings write's queued task ran). Reading `docRef.current` at execution time closes that race the same way `applyOp` itself does. `useDocSettings.test.ts` pins the exact repro, plus a negative control proving the old (call-time) shape actually reproduces the corruption.

### Used by

`Editor` (`src/editor/Editor.tsx`) - `saveDocSettings` is passed to `EditorPanels` (and every settings panel beneath it); `onAutoModel` is passed through to `AiPanel` alone. `enqueue` is `Editor`'s own `createQueue()` instance, shared with `applyOp`/`undo`/`redo`/`onRun`; `docRef` is `Editor`'s own `docRef`, shared with all of those too.
