import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { truncateToastText } from "./toastText";

const DISMISS_MS = 1600;

export interface ToastMsg { id: number; text: string }

/** Owns the pill's state so `Editor` only has to wire `onSwap` into `useEditHistory` and render
 *  `<Toast msg={msg} onDone={dismiss} />` - it doesn't need its own `useState`/`useRef` for this.
 *  `id` bumps on every fire (even the same text twice in a row) so `Toast`'s `AnimatePresence` key
 *  always remounts and restarts the dismiss timer instead of two dismissals racing. `push` is the
 *  general form (any text - Task 11 reuses it for the `export-warning` IPC event, see
 *  useExportState.ts); `onSwap` is just `push` with undo/redo's own two fixed strings, kept as its
 *  own name since that's the call site `useEditHistory`'s `onSwap` prop already expects. */
export function useUndoToast() {
  const [msg, setMsg] = useState<ToastMsg | null>(null);
  const idRef = useRef(0);
  const push = useCallback((text: string) => {
    idRef.current += 1;
    setMsg({ id: idRef.current, text: truncateToastText(text) });
  }, []);
  const onSwap = useCallback((kind: "undo" | "redo") => push(kind === "undo" ? "Undid" : "Redid"), [push]);
  const dismiss = useCallback(() => setMsg(null), []);
  return { msg, onSwap, push, dismiss };
}

/** A minimal, auto-dismissing feedback pill - originally fired only after undo/redo (see
 *  `Editor`'s `onSwap`), so the user gets a beat of confirmation for a keyboard-triggered
 *  Ctrl+Z/Ctrl+Shift+Z that has no other visible feedback (unlike the TopBar buttons, which
 *  already show a state change via `canUndo`/`canRedo`); now also the export-warning surface (see
 *  `useUndoToast`'s `push` above). `msg.id` is bumped on every fire (even for the same text twice
 *  in a row) so `AnimatePresence`'s `key` always remounts and restarts the dismiss timer, rather
 *  than the second toast silently reusing the first one's clock. */
export function Toast({ msg, onDone }: { msg: ToastMsg | null; onDone: () => void }) {
  useEffect(() => {
    if (!msg) return;
    const t = setTimeout(onDone, DISMISS_MS);
    return () => clearTimeout(t);
  }, [msg, onDone]);

  return (
    <AnimatePresence>
      {msg && (
        <motion.div key={msg.id} className="e-toast"
          initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 8 }}
          transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
          {msg.text}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
