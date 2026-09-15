import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { IconCheck } from "@tabler/icons-react";

const PRESS_TAP = { scale: 0.96 };
const PRESS_SPRING = { type: "spring" as const, stiffness: 500, damping: 30 };

export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel,
  danger,
  dontAskLabel,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  body: string;
  confirmLabel: string;
  danger?: boolean;
  dontAskLabel?: string;
  onConfirm: (dontAsk: boolean) => void;
  onCancel: () => void;
}) {
  const [dontAsk, setDontAsk] = useState(false);
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="e-modal-scrim"
          onPointerDown={onCancel}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.14 }}
        >
          <motion.div
            className="e-modal"
            onPointerDown={(e) => e.stopPropagation()}
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}
          >
            <h3 className="e-modal-title">{title}</h3>
            <p className="e-modal-body">{body}</p>
            {dontAskLabel && (
              <label className="e-modal-dontask">
                <input
                  type="checkbox"
                  checked={dontAsk}
                  onChange={(e) => setDontAsk(e.target.checked)}
                  style={{ position: "absolute", opacity: 0, width: 0, height: 0 }}
                />
                <span
                  style={{
                    width: 17,
                    height: 17,
                    flex: "none",
                    borderRadius: 5,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    transition: "background .12s ease, border-color .12s ease",
                    border: `1px solid ${dontAsk ? "var(--e-primary)" : "var(--e-border2)"}`,
                    background: dontAsk ? "var(--e-primary)" : "var(--e-soft)",
                  }}
                >
                  {dontAsk && <IconCheck size={11} stroke={3.5} color="var(--e-on-fill)" />}
                </span>
                <span>{dontAskLabel}</span>
              </label>
            )}
            <div className="e-modal-actions">
              <button type="button" className="e-modal-btn" onClick={onCancel}>
                Cancel
              </button>
              <motion.button
                type="button"
                className={`e-modal-btn primary${danger ? " danger" : ""}`}
                onClick={() => onConfirm(dontAsk)}
                whileTap={PRESS_TAP}
                transition={PRESS_SPRING}
              >
                {confirmLabel}
              </motion.button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
