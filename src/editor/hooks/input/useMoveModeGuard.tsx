import { useCallback, useState, type RefObject } from "react";
import type { EditDoc, EditOp } from "../../../shared/edit";
import type { CamPose } from "../../stage/camera/cameraMoves";
import { ConfirmDialog } from "../../controls/surfaces/ConfirmDialog";

const SUPPRESS_KEY = "tcursor_hide_move_off_warn";

export function useMoveModeGuard(
  doc: EditDoc | null,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
  camDraftRef: RefObject<CamPose | null>,
) {
  const [moveMode, setMoveMode] = useState(false);
  const [pending, setPending] = useState(false);
  const off = useCallback(() => {
    camDraftRef.current = null;
    setMoveMode(false);
  }, [camDraftRef]);

  const clearKeyframes = useCallback(async () => {
    if (!doc) return;
    for (const m of doc.camera_moves) await applyOp({ op: "remove_camera_move", id: m.id });
  }, [doc, applyOp]);

  const requestMoveMode = useCallback(
    (next: boolean) => {
      const hasKf = (doc?.camera_moves.length ?? 0) > 0;
      if (!next && hasKf) {
        if (localStorage.getItem(SUPPRESS_KEY) === "1") {
          void clearKeyframes();
          off();
        } else setPending(true);
      } else if (next) {
        setMoveMode(true);
      } else {
        off();
      }
    },
    [doc, clearKeyframes, off],
  );

  const confirm = useCallback(
    (dontAsk: boolean) => {
      if (dontAsk) localStorage.setItem(SUPPRESS_KEY, "1");
      void clearKeyframes();
      off();
      setPending(false);
    },
    [clearKeyframes, off],
  );

  const n = doc?.camera_moves.length ?? 0;
  const moveOffDialog = (
    <ConfirmDialog
      open={pending}
      title="Turn off Move mode?"
      danger
      body={`Your ${n} camera keyframe${n === 1 ? "" : "s"} will be removed so the static webcam controls take over again.`}
      confirmLabel="Remove & turn off"
      dontAskLabel="Don't ask again"
      onConfirm={confirm}
      onCancel={() => setPending(false)}
    />
  );

  return { moveMode, requestMoveMode, moveOffDialog, moveOffOpen: pending };
}
