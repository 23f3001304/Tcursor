import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { saveEdit } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";

const LIMIT = 100; // bounded undo depth
const COALESCE_MS = 400; // merge a rapid burst (e.g. a slider drag firing many ops) into ONE undo step

// Frontend edit history for the EditDoc. `record(current)` snapshots the pre-edit doc before every
// mutation; undo/redo swap between the two stacks, persist the restored doc (saveEdit) and bump
// `rev` so the preview refetches. No backend history is needed - the doc IS the whole edit state.
//
// `enqueue` (from `Editor`'s `createQueue()`, shared with `applyOp`) serializes `undo`/`redo`
// behind any in-flight `applyOp` - without it, an undo/redo issued while an edit's IPC round trip
// is still pending could `setDoc` the restored snapshot, only for the apply's OWN `setDoc` to
// land after and silently overwrite it (UI and disk diverge). Routing through the same queue here
// (not just at the call sites in `Editor`) also covers this hook's OWN `Ctrl+Z` listener below,
// which calls these same `undo`/`redo` functions.
export function useEditHistory(
  folder: string,
  docRef: RefObject<EditDoc | null>,
  setDoc: (d: EditDoc) => void,
  bumpRev: () => void,
  enqueue: <T>(fn: () => Promise<T>) => Promise<T>,
  onSwap?: (kind: "undo" | "redo") => void,
) {
  const undoStack = useRef<EditDoc[]>([]);
  const redoStack = useRef<EditDoc[]>([]);
  const lastAt = useRef(0);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const sync = () => {
    setCanUndo(undoStack.current.length > 0);
    setCanRedo(redoStack.current.length > 0);
  };

  // Call BEFORE applying an edit, with the CURRENT (pre-edit) doc.
  const record = useCallback((current: EditDoc) => {
    const now = Date.now();
    // Only push a new step when the gap since the last edit exceeds COALESCE_MS (or the stack is
    // empty). A slider drag then collapses to the single pre-drag snapshot -> one undo reverts it.
    if (undoStack.current.length === 0 || now - lastAt.current > COALESCE_MS) {
      undoStack.current.push(current);
      if (undoStack.current.length > LIMIT) undoStack.current.shift();
    }
    lastAt.current = now;
    redoStack.current = []; // a fresh edit invalidates the redo trail
    sync();
  }, []);

  // Reads `docRef.current` (not a captured `doc` value) at the moment this actually RUNS - since
  // `swap` only ever executes from inside `enqueue` (via `undo`/`redo` below), a call queued
  // behind an in-flight `applyOp` must see whatever doc that apply just settled to, not whatever
  // was current back when the undo/redo was first triggered. A closure-captured `doc` would push
  // that stale pre-apply snapshot as "the doc we're leaving" - the wrong doc ends up on the
  // counterpart stack, and a later redo/undo would silently discard the apply's real result.
  const swap = useCallback(async (from: { current: EditDoc[] }, to: { current: EditDoc[] }, kind: "undo" | "redo") => {
    const next = from.current.pop();
    const current = docRef.current;
    if (!next || !current) return; // nothing to swap - no toast, no write
    to.current.push(current); // the doc we're leaving becomes the counterpart step
    setDoc(next);
    lastAt.current = 0; // the next edit after undo/redo always starts its own step
    try { await saveEdit(folder, next); } catch { /* keep the UI state even if the disk write fails */ }
    bumpRev();
    sync();
    onSwap?.(kind);
  }, [docRef, folder, setDoc, bumpRev, onSwap]);

  const undo = useCallback(() => enqueue(() => swap(undoStack, redoStack, "undo")), [swap, enqueue]);
  const redo = useCallback(() => enqueue(() => swap(redoStack, undoStack, "redo")), [swap, enqueue]);

  // Ctrl/Cmd+Z = undo, Ctrl/Cmd+Shift+Z or Ctrl+Y = redo. Ignored while typing in a field so the
  // input's own undo still works.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) { e.preventDefault(); void undo(); }
      else if ((k === "z" && e.shiftKey) || k === "y") { e.preventDefault(); void redo(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo, redo]);

  return { record, undo, redo, canUndo, canRedo };
}
