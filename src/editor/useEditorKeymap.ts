import { useEffect } from "react";
import type { EditDoc, EditOp } from "../lib/edit";

// Global keyboard shortcuts for the editor: Delete/Backspace removes the selected
// zoom/effect/layout segment, Z/S add a zoom/spotlight at the playhead, Space toggles play.
// Skipped while an input/textarea/contenteditable is focused.
export function useEditorKeymap(opts: {
  sel: string | null;
  doc: EditDoc | null;
  timeMs: number;
  setSel: (id: string | null) => void;
  setPlaying: (fn: (p: boolean) => boolean) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  addZoom: () => Promise<void>;
  addSpotlight: () => Promise<void>;
}) {
  const { sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight } = opts;
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === "INPUT" || activeEl.tagName === "TEXTAREA" || activeEl.getAttribute("contenteditable") === "true")) {
        return;
      }
      if (e.key === "Delete" || e.key === "Backspace") {
        if (sel) {
          const isZoom = doc?.zooms.some((z) => z.id === sel);
          const isEffect = doc?.effects.some((f) => f.id === sel);
          const isLayout = doc?.layout.some((l) => l.id === sel);
          if (isZoom) {
            void applyOp({ op: "remove_zoom", id: sel });
            setSel(null);
          } else if (isEffect) {
            void applyOp({ op: "remove_effect", id: sel });
            setSel(null);
          } else if (isLayout) {
            void applyOp({ op: "remove_layout_seg", id: sel });
            setSel(null);
          }
        }
      } else if (e.key.toLowerCase() === "z") {
        void addZoom();
      } else if (e.key.toLowerCase() === "s") {
        void addSpotlight();
      } else if (e.key === " ") {
        e.preventDefault();
        setPlaying((p) => !p);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [sel, doc, timeMs]);
}
