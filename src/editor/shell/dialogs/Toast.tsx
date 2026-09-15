import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { truncateToastText } from "./toastText";

const DISMISS_MS = 1600;

export interface ToastMsg {
  id: number;
  text: string;
}

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

export function Toast({ msg, onDone }: { msg: ToastMsg | null; onDone: () => void }) {
  useEffect(() => {
    if (!msg) return;
    const t = setTimeout(onDone, DISMISS_MS);
    return () => clearTimeout(t);
  }, [msg, onDone]);

  return (
    <AnimatePresence>
      {msg && (
        <motion.div
          key={msg.id}
          className="e-toast"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 8 }}
          transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
        >
          {msg.text}
        </motion.div>
      )}
    </AnimatePresence>
  );
}
