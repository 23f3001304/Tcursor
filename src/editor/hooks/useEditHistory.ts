import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { saveEdit } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";

const LIMIT = 100; // bounded undo depth
const COALESCE_MS = 400; // merge a rapid burst (e.g. a slider drag firing many ops) into ONE undo step

/** Pure: should `record` push a NEW snapshot (vs. coalesce into the top of an already-pending
 *  one)? Extracted so the coalescing window's exact edges are unit-testable without mounting the
 *  hook. Mirrors `record`'s own condition exactly: empty stack, or the last push was more than
 *  `coalesceMs` ago. */
export function shouldPushNewSnapshot(stackLen: number, lastAt: number, now: number, coalesceMs: number = COALESCE_MS): boolean {
  return stackLen === 0 || now - lastAt > coalesceMs;
}

/** Pure: is `snapshot` still exactly the top of `stack`? The identity guard `unrecord` needs so a
 *  stale/duplicate call (or one that lost a race it should never have been able to lose - see
 *  `useEditHistory.md`) can never pop a DIFFERENT, real edit's snapshot - only the one it actually
 *  pushed, and only while nothing has pushed on top of it since. */
export function matchesStackTop<T>(stack: T[], snapshot: T | null): boolean {
  return snapshot !== null && stack.length > 0 && stack[stack.length - 1] === snapshot;
}

/** Everything `record` hands back for `unrecord` to fully roll itself back with: the snapshot it
 *  pushed (`null` when coalesced - nothing to roll back), and the PRE-CALL `lastAt`/redo-stack to
 *  restore verbatim, so undoing a failed edit's bookkeeping doesn't also poison the coalescing
 *  window or discard a real redo trail that `record` cleared for an edit that never happened. */
export interface RecordToken { snapshot: EditDoc | null; prevLastAt: number; prevRedo: EditDoc[] }

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

  // Call BEFORE applying an edit, with the CURRENT (pre-edit) doc. See `RecordToken` above for
  // what the return carries and why `unrecord` needs all of it, not just a boolean.
  const record = useCallback((current: EditDoc): RecordToken => {
    const now = Date.now();
    const prevLastAt = lastAt.current;
    const prevRedo = redoStack.current;
    let snapshot: EditDoc | null = null;
    if (shouldPushNewSnapshot(undoStack.current.length, prevLastAt, now)) {
      undoStack.current.push(current);
      if (undoStack.current.length > LIMIT) undoStack.current.shift();
      snapshot = current;
    }
    lastAt.current = now;
    redoStack.current = []; // a fresh edit invalidates the redo trail (restored in `unrecord` if this call is undone)
    sync();
    return { snapshot, prevLastAt, prevRedo };
  }, []);

  // Call when an apply that just called `record()` and got a non-null `token.snapshot` back then
  // FAILS - pops that snapshot back off (identity-checked via `matchesStackTop`, NOT a blind pop:
  // a record from a DIFFERENT op landing on top in between must never be the one that gets popped)
  // and restores `lastAt`/the redo stack to their pre-`record` values, so a rejected op never
  // leaves a phantom undo step, a poisoned coalescing window, or a redo trail wiped for nothing.
  const unrecord = useCallback((token: RecordToken) => {
    if (!matchesStackTop(undoStack.current, token.snapshot)) return;
    undoStack.current.pop();
    lastAt.current = token.prevLastAt;
    redoStack.current = token.prevRedo;
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
    // Written immediately, same reason `applyOp` does (Editor.md/M1): a SECOND queued undo/redo
    // runs as a microtask the instant THIS one settles, before React's own re-render lands - it
    // must see the doc this swap actually landed on, not the one it left.
    docRef.current = next;
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

  return { record, unrecord, undo, redo, canUndo, canRedo };
}
