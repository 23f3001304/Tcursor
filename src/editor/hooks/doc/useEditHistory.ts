import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import { saveEdit } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";

const LIMIT = 100;
const COALESCE_MS = 400;

export function shouldPushNewSnapshot(
  stackLen: number,
  lastAt: number,
  now: number,
  coalesceMs: number = COALESCE_MS,
): boolean {
  return stackLen === 0 || now - lastAt > coalesceMs;
}

export function matchesStackTop<T>(stack: T[], snapshot: T | null): boolean {
  return snapshot !== null && stack.length > 0 && stack[stack.length - 1] === snapshot;
}

export interface RecordToken {
  snapshot: EditDoc | null;
  prevLastAt: number;
  prevRedo: EditDoc[];
}

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
    redoStack.current = [];
    sync();
    return { snapshot, prevLastAt, prevRedo };
  }, []);

  const unrecord = useCallback((token: RecordToken) => {
    if (!matchesStackTop(undoStack.current, token.snapshot)) return;
    undoStack.current.pop();
    lastAt.current = token.prevLastAt;
    redoStack.current = token.prevRedo;
    sync();
  }, []);

  const swap = useCallback(
    async (from: { current: EditDoc[] }, to: { current: EditDoc[] }, kind: "undo" | "redo") => {
      const next = from.current.pop();
      const current = docRef.current;
      if (!next || !current) return;
      to.current.push(current);
      setDoc(next);
      docRef.current = next;
      lastAt.current = 0;
      try {
        await saveEdit(folder, next);
      } catch {}
      bumpRev();
      sync();
      onSwap?.(kind);
    },
    [docRef, folder, setDoc, bumpRev, onSwap],
  );

  const undo = useCallback(() => enqueue(() => swap(undoStack, redoStack, "undo")), [swap, enqueue]);
  const redo = useCallback(() => enqueue(() => swap(redoStack, undoStack, "redo")), [swap, enqueue]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      const k = e.key.toLowerCase();
      if (k === "z" && !e.shiftKey) {
        e.preventDefault();
        void undo();
      } else if ((k === "z" && e.shiftKey) || k === "y") {
        e.preventDefault();
        void redo();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undo, redo]);

  return { record, unrecord, undo, redo, canUndo, canRedo };
}
