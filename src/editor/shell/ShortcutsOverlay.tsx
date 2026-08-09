import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";

const SHORTCUTS: { keys: string; label: string }[] = [
  { keys: "Space", label: "Play / Pause" },
  { keys: "Z", label: "Add zoom" },
  { keys: "S", label: "Add spotlight" },
  { keys: "Del", label: "Remove selected" },
  { keys: "Ctrl+Z", label: "Undo" },
  { keys: "Ctrl+Shift+Z", label: "Redo" },
  { keys: "?", label: "Shortcuts" },
];

/** Small centered modal listing the editor's keyboard shortcuts - opened via `?` (`useEditorKeymap`'s
 *  "overlay" action, see `keymap.ts`). Reuses the shared `.e-modal`/`.e-modal-scrim` pattern
 *  (`ConfirmDialog`/`ExportDialog`) with the same 0.16s tween, so it reads as the same modal family
 *  rather than a bespoke popup. Esc or a scrim click both dismiss. */
export function ShortcutsOverlay({ open, onClose }: { open: boolean; onClose: () => void }) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div className="e-modal-scrim" onPointerDown={onClose}
          initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.14 }}>
          <motion.div className="e-modal e-shortcuts-modal" onPointerDown={(e) => e.stopPropagation()}
            initial={{ opacity: 0, scale: 0.96, y: 8 }} animate={{ opacity: 1, scale: 1, y: 0 }} exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ type: "tween", duration: 0.16, ease: [0.4, 0, 0.2, 1] }}>
            <h3 className="e-modal-title">Keyboard shortcuts</h3>
            <ul className="e-shortcuts-list">
              {SHORTCUTS.map((s) => (
                <li key={s.label}>
                  <span className="e-shortcuts-label">{s.label}</span>
                  <kbd className="e-shortcuts-key">{s.keys}</kbd>
                </li>
              ))}
            </ul>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
