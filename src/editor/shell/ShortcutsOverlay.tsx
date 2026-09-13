import { useEffect } from "react";
import { AnimatePresence, motion } from "motion/react";

/** Two groups, because the editor now has two kinds of key: the ones that change the recording and
 *  the ones that change the shell it is being edited in. Every KEY here is a row of the one keymap
 *  table (`hooks/keymap.ts`), so this list cannot drift into describing a binding that does not
 *  exist - if a binding moves, the table is what moved. The one exception is the Shift+drag range
 *  gesture (`timeline/useRangeSelect.ts`), which is a pointer gesture and so has no table row; it
 *  is listed anyway because it is the only way to reach the transport's Cut and Speed precisely. */
const GROUPS: { name: string; keys: { keys: string; label: string }[] }[] = [
  {
    name: "Editing",
    keys: [
      { keys: "Space", label: "Play / Pause" },
      { keys: "Z", label: "Add zoom" },
      { keys: "S", label: "Add spotlight" },
      { keys: "Shift+drag", label: "Choose a range on the ruler" },
      { keys: "Del", label: "Remove selected" },
      { keys: "Esc", label: "Deselect" },
      { keys: "Ctrl+Z", label: "Undo" },
      { keys: "Ctrl+Shift+Z", label: "Redo" },
      { keys: "?", label: "Shortcuts" },
    ],
  },
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
            {GROUPS.map((group) => (
              <section key={group.name}>
                <h4 className="e-shortcuts-group">{group.name}</h4>
                <ul className="e-shortcuts-list">
                  {group.keys.map((s) => (
                    <li key={s.label}>
                      <span className="e-shortcuts-label">{s.label}</span>
                      <kbd className="e-shortcuts-key">{s.keys}</kbd>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
