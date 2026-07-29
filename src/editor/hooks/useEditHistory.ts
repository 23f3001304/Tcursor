import { useCallback, useEffect, useRef, useState } from "react";
import { saveEdit } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";

const LIMIT = 100; // bounded undo depth
const COALESCE_MS = 400; // merge a rapid burst (e.g. a slider drag firing many ops) into ONE undo step

// Frontend edit history for the EditDoc. `record(current)` snapshots the pre-edit doc before every
// mutation; undo/redo swap between the two stacks, persist the restored doc (saveEdit) and bump
// `rev` so the preview refetches. No backend history is needed - the doc IS the whole edit state.
export function useEditHistory(
  folder: string,
  doc: EditDoc | null,
  setDoc: (d: EditDoc) => void,
  bumpRev: () => void,
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

  const swap = useCallback(async (from: { current: EditDoc[] }, to: { current: EditDoc[] }) => {
    const next = from.current.pop();
    if (!next || !doc) return;
    to.current.push(doc); // the doc we're leaving becomes the counterpart step
    setDoc(next);
    lastAt.current = 0; // the next edit after undo/redo always starts its own step
    try { await saveEdit(folder, next); } catch { /* keep the UI state even if the disk write fails */ }
    bumpRev();
    sync();
  }, [doc, folder, setDoc, bumpRev]);

  const undo = useCallback(() => swap(undoStack, redoStack), [swap]);
  const redo = useCallback(() => swap(redoStack, undoStack), [swap]);

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
