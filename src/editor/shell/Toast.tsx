import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";

const DISMISS_MS = 1600;

export interface ToastMsg { id: number; text: string }

/** Owns the pill's state so `Editor` only has to wire `onSwap` into `useEditHistory` and render
 *  `<Toast msg={msg} onDone={dismiss} />` - it doesn't need its own `useState`/`useRef` for this.
 *  `id` bumps on every fire (even repeat "Undid" in a row) so `Toast`'s `AnimatePresence` key
 *  always remounts and restarts the dismiss timer instead of two dismissals racing. */
export function useUndoToast() {
  const [msg, setMsg] = useState<ToastMsg | null>(null);
  const idRef = useRef(0);
  const onSwap = useCallback((kind: "undo" | "redo") => {
    idRef.current += 1;
    setMsg({ id: idRef.current, text: kind === "undo" ? "Undid" : "Redid" });
  }, []);
  const dismiss = useCallback(() => setMsg(null), []);
  return { msg, onSwap, dismiss };
}

/** A minimal, auto-dismissing feedback pill - currently only fired after undo/redo (see
 *  `Editor`'s `onSwap`), so the user gets a beat of confirmation for a keyboard-triggered
 *  Ctrl+Z/Ctrl+Shift+Z that has no other visible feedback (unlike the TopBar buttons, which
 *  already show a state change via `canUndo`/`canRedo`). `msg.id` is bumped on every fire (even
 *  for the same text twice in a row) so `AnimatePresence`'s `key` always remounts and restarts
 *  the dismiss timer, rather than the second toast silently reusing the first one's clock. */
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
