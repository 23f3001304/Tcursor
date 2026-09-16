import { useEffect } from "react";
import type { EditDoc, EditOp, TextKind } from "../../../shared/edit";
import { resolveKeyAction, type TargetLike } from "../../model/keymap";

export function useEditorKeymap(opts: {
  sel: string | null;
  doc: EditDoc | null;
  timeMs: number;
  setSel: (id: string | null) => void;
  setPlaying: (fn: (p: boolean) => boolean) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  addZoom: () => Promise<void>;
  addSpotlight: () => Promise<void>;
  addText: (kind: TextKind) => Promise<void>;
  onOverlay: () => void;
  modalOpen: boolean;
  shortcutsOpen: boolean;
  escOwned: boolean;
}) {
  const {
    sel,
    doc,
    timeMs,
    setSel,
    setPlaying,
    applyOp,
    addZoom,
    addSpotlight,
    addText,
    onOverlay,
    modalOpen,
    shortcutsOpen,
    escOwned,
  } = opts;
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const activeEl = document.activeElement;
      const target: TargetLike = activeEl
        ? {
            tagName: activeEl.tagName,
            role: activeEl.getAttribute("role"),
            isContentEditable: (activeEl as HTMLElement).isContentEditable === true,
          }
        : { tagName: "", role: null, isContentEditable: false };
      switch (resolveKeyAction(e, { hasSel: !!sel, modalOpen, shortcutsOpen, target })) {
        case "delete": {
          if (!sel) break;
          const isZoom = doc?.zooms.some((z) => z.id === sel);
          const isEffect = doc?.effects.some((f) => f.id === sel);
          const isLayout = doc?.layout.some((l) => l.id === sel);
          const isCamMove = doc?.camera_moves.some((m) => m.id === sel);
          const isCut = doc?.cuts.some((c) => c.id === sel);
          const isSpeed = doc?.speed.some((s) => s.id === sel);
          const isText = doc?.texts?.some((t) => t.id === sel);
          if (isZoom) void applyOp({ op: "remove_zoom", id: sel });
          else if (isEffect) void applyOp({ op: "remove_effect", id: sel });
          else if (isLayout) void applyOp({ op: "remove_layout_seg", id: sel });
          else if (isCamMove) void applyOp({ op: "remove_camera_move", id: sel });
          else if (isCut) void applyOp({ op: "remove_cut", id: sel });
          else if (isSpeed) void applyOp({ op: "remove_speed", id: sel });
          else if (isText) void applyOp({ op: "remove_text", id: sel });
          else break;
          setSel(null);
          break;
        }
        case "deselect":
          if (!escOwned) setSel(null);
          break;
        case "zoom":
          void addZoom();
          break;
        case "spotlight":
          void addSpotlight();
          break;
        case "text":
          void addText("title");
          break;
        case "play":
          e.preventDefault();
          setPlaying((p) => !p);
          break;
        case "overlay":
          e.preventDefault();
          onOverlay();
          break;
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [sel, doc, timeMs, modalOpen, shortcutsOpen, escOwned]);
}
