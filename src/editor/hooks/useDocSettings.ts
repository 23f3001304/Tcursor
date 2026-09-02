import { useCallback, type RefObject } from "react";
import { saveEdit } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";
import type { RecordToken } from "./useEditHistory";

/** Settings-only doc writes, split out of `Editor` (bulk `settings` patches, as opposed to
 *  `applyOp`'s per-`EditOp` mutations). Both flavors share one underlying write (`write`):
 *  `saveDocSettings` records an undo step first - the normal path for every settings-editing
 *  panel (background, cursor, camera, captions, audio, the AI model picker). `onAutoModel` skips
 *  `record()` - it's used ONLY by `AiPanel`'s mount-time "default to an installed Ollama model"
 *  effect, so landing on that default the instant the panel opens never pushes a phantom undo
 *  step or an unasked-for disk write purely from mounting.
 *
 *  `write` is routed through the SAME `enqueue` `Editor` serializes `applyOp`/`undo`/`redo`
 *  through (D-Medium M5) - `save_edit` (a blind full-doc overwrite) and `apply_edit_op` (a
 *  load-mutate-save) are last-writer-wins over each other on disk if they overlap, and a settings
 *  slider fires ~60 of these a second while dragging.
 *
 *  Both `record` and the base doc it merges `nextSettings` into are read from `docRef.current`
 *  INSIDE the enqueued task, not from a `doc` value closed over at call time (review round 1,
 *  Critical 1): a settings write queued behind an in-flight `applyOp` used to close over
 *  whatever `doc` was at the moment the user clicked, which - since the queue can delay
 *  execution - is stale by the time it actually runs, and would blind-overwrite the apply's
 *  result (`{...staleDoc, settings: next}` drops every non-settings field the apply just
 *  changed) on both disk and in `setDoc`. Reading `docRef.current` at execution time closes
 *  that race the same way `applyOp` itself does. */
export function useDocSettings(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  record: (current: EditDoc) => RecordToken,
  setRev: (fn: (r: number) => number) => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
) {
  const write = useCallback((nextSettings: EditDoc["settings"], recordUndo: boolean) => enqueue(async () => {
    const doc = docRef.current;
    if (!doc) return;
    if (recordUndo) record(doc); // AT EXECUTION TIME - the doc this settings change is actually landing on
    const newDoc = { ...doc, settings: nextSettings };
    setDoc(newDoc); docRef.current = newDoc; // read by a queued undo/redo BEFORE React's own re-render lands
    // Matches `useEditHistory.swap`'s own precedent: keep the optimistic UI even if the disk
    // write fails, rather than an uncaught rejection wedging `rev` (and every settings-panel
    // caller) forever - see the failure scenario in M5.
    try { await saveEdit(folder, newDoc); } catch { /* keep the UI state even if the disk write fails */ }
    setRev((r) => r + 1);
  }), [docRef, folder, record, setDoc, setRev, enqueue]);

  const saveDocSettings = useCallback((nextSettings: EditDoc["settings"]) => write(nextSettings, true), [write]);

  const onAutoModel = useCallback((v: string) => {
    const doc = docRef.current;
    if (doc) void write({ ...doc.settings, ai_model: v }, false);
  }, [docRef, write]);

  return { saveDocSettings, onAutoModel };
}
