import { useEffect } from "react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { resolveKeyAction, type TargetLike } from "./keymap";

// Global keyboard shortcuts for the editor: Delete/Backspace removes the selected
// zoom/effect/layout segment, Z/S add a zoom/spotlight at the playhead, Space toggles play,
// ? opens (or, while it's the current modal, closes) the shortcuts overlay. Inert while typing in
// a field, while a modal is open (`modalOpen`), or - for Space specifically - while the focused
// element owns Space itself (a button, or a custom control like Switch/Picker that manages its
// own keydown; M4). The actual decision is `resolveKeyAction` (`keymap.ts`, unit-tested) - this
// hook just gathers the DOM context and switches on its result.
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
  modalOpen: boolean;
  /** Whether `ShortcutsOverlay` itself is the (or a) currently-open modal - lets `resolveKeyAction`
   *  still let `?` through to toggle it CLOSED even while `modalOpen` is true because of it. */
  shortcutsOpen: boolean;
}) {
  const { sel, doc, timeMs, setSel, setPlaying, applyOp, addZoom, addSpotlight, onOverlay, modalOpen, shortcutsOpen } = opts;
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      const target: TargetLike = activeEl
        ? { tagName: activeEl.tagName, role: activeEl.getAttribute("role"), isContentEditable: (activeEl as HTMLElement).isContentEditable === true }
        : { tagName: "", role: null, isContentEditable: false };
      switch (resolveKeyAction(e, { hasSel: !!sel, modalOpen, shortcutsOpen, target })) {
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
  }, [sel, doc, timeMs, modalOpen, shortcutsOpen]);
}
