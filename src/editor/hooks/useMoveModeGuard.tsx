import { useCallback, useState } from "react";
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

  // `useCallback`'d (render hygiene pass) so `requestMoveMode` stays referentially stable across
  // renders that don't touch `doc`/`applyOp` (e.g. a playhead tick) - it's passed to `EditorPanels`
  // (`React.memo`'d), which needs that to actually skip re-rendering. `doc`/`applyOp` themselves
  // don't change identity on a tick either, so this chain holds without any ref-reading tricks.
  const clearKeyframes = useCallback(async () => {
    if (!doc) return;
    for (const m of doc.camera_moves) await applyOp({ op: "remove_camera_move", id: m.id });
  }, [doc, applyOp]);

  const requestMoveMode = useCallback((next: boolean) => {
    const hasKf = (doc?.camera_moves.length ?? 0) > 0;
    if (!next && hasKf) {
      if (localStorage.getItem(SUPPRESS_KEY) === "1") { void clearKeyframes(); setMoveMode(false); }
      else setPending(true);
    } else {
      setMoveMode(next);
    }
  }, [doc, clearKeyframes]);

  const confirm = useCallback((dontAsk: boolean) => {
    if (dontAsk) localStorage.setItem(SUPPRESS_KEY, "1");
    void clearKeyframes(); setMoveMode(false); setPending(false);
  }, [clearKeyframes]);

  const n = doc?.camera_moves.length ?? 0;
  const moveOffDialog = (
    <ConfirmDialog open={pending} title="Turn off Move mode?" danger
      body={`Your ${n} camera keyframe${n === 1 ? "" : "s"} will be removed so the static webcam controls take over again.`}
      confirmLabel="Remove & turn off" dontAskLabel="Don't ask again"
      onConfirm={confirm} onCancel={() => setPending(false)} />
  );

  // `pending` re-exposed as `moveOffOpen` - lets `Editor` fold this ConfirmDialog into its
  // "is any modal open" check for `useEditorKeymap` (Space/Z/S must be inert behind it too).
  return { moveMode, requestMoveMode, moveOffDialog, moveOffOpen: pending };
}
