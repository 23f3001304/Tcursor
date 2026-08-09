import { useEffect } from "react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { keyAction } from "./keymap";

// Global keyboard shortcuts for the editor: Delete/Backspace removes the selected
// zoom/effect/layout segment, Z/S add a zoom/spotlight at the playhead, Space toggles play,
// ? opens the shortcuts overlay. Skipped while an input/textarea/contenteditable is focused.
// The actual key->action decision is `keyAction` (`keymap.ts`, unit-tested) - this hook just
// switches on its result and performs the corresponding op.
export function useEditorKeymap(opts: {
  sel: string | null;
  doc: EditDoc | null;
  timeMs: number;
  setSel: (id: string | null) => void;
  setPlaying: (fn: (p: boolean) => boolean) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  addZoom: () => Promise<void>;
  addSpotlight: () => Promise<void>;
  onOverlay: () => void;
}) {
  const { sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight, onOverlay } = opts;
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      if (activeEl && (activeEl.tagName === "INPUT" || activeEl.tagName === "TEXTAREA" || activeEl.getAttribute("contenteditable") === "true")) {
        return;
      }
      switch (keyAction(e, !!sel)) {
        case "delete": {
          if (!sel) break;
          const isZoom = doc?.zooms.some((z) => z.id === sel);
          const isEffect = doc?.effects.some((f) => f.id === sel);
          const isLayout = doc?.layout.some((l) => l.id === sel);
          const isCamMove = doc?.camera_moves.some((m) => m.id === sel);
          if (isZoom) void applyOp({ op: "remove_zoom", id: sel });
          else if (isEffect) void applyOp({ op: "remove_effect", id: sel });
          else if (isLayout) void applyOp({ op: "remove_layout_seg", id: sel });
          else if (isCamMove) void applyOp({ op: "remove_camera_move", id: sel });
          else break; // sel points at nothing currently in the doc - nothing to clear
          setSel(null);
          break;
        }
        case "zoom": void addZoom(); break;
        case "spotlight": void addSpotlight(); break;
        case "play": e.preventDefault(); setPlaying((p) => !p); break;
        case "overlay": e.preventDefault(); onOverlay(); break;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [sel, doc, timeMs]);
}
