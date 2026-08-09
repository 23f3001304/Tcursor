import { useState } from "react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { ConfirmDialog } from "../controls/ConfirmDialog";

const SUPPRESS_KEY = "tcursor_hide_move_off_warn";

/** Owns the "Move in preview" toggle and guards turning it OFF. camera_moves keyframes override the
 *  static webcam controls (size/dock) within the span they own (Task 27), so switching back to
 *  static only fully takes effect if the keyframes are cleared - otherwise the static sliders look
 *  dead across that span while working everywhere else. Turning Move off therefore
 *  removes the keyframes, but warns first (unless the user ticked "don't ask again") so they are
 *  not lost by accident. Returns the current mode, the toggle handler, and the dialog to render. */
export function useMoveModeGuard(doc: EditDoc | null, applyOp: (op: EditOp) => Promise<EditDoc | null>) {
  const [moveMode, setMoveMode] = useState(false);
  const [pending, setPending] = useState(false);

  const clearKeyframes = async () => {
    if (!doc) return;
    for (const m of doc.camera_moves) await applyOp({ op: "remove_camera_move", id: m.id });
  };

  const requestMoveMode = (next: boolean) => {
    const hasKf = (doc?.camera_moves.length ?? 0) > 0;
    if (!next && hasKf) {
      if (localStorage.getItem(SUPPRESS_KEY) === "1") { void clearKeyframes(); setMoveMode(false); }
      else setPending(true);
    } else {
      setMoveMode(next);
    }
  };

  const confirm = (dontAsk: boolean) => {
    if (dontAsk) localStorage.setItem(SUPPRESS_KEY, "1");
    void clearKeyframes(); setMoveMode(false); setPending(false);
  };

  const n = doc?.camera_moves.length ?? 0;
  const moveOffDialog = (
    <ConfirmDialog open={pending} title="Turn off Move mode?" danger
      body={`Your ${n} camera keyframe${n === 1 ? "" : "s"} will be removed so the static webcam controls take over again.`}
      confirmLabel="Remove & turn off" dontAskLabel="Don't ask again"
      onConfirm={confirm} onCancel={() => setPending(false)} />
  );

  return { moveMode, requestMoveMode, moveOffDialog };
}
