import type { RefObject } from "react";
import { PanelHeader } from "../PanelHeader";
import { CameraMoveField } from "./CameraMoveField";
import type { AppearanceSettings } from "../../../hud/settings/settings";
import type { EditDoc, EditOp } from "../../../shared/edit";
import type { CamPose } from "../../stage/camera/cameraMoves";
import { DEFAULT_APPEARANCE } from "../../../hud/preferences/appearanceFields";

export function CameraPanel({
  settings,
  onClose,
  doc,
  timeMs,
  applyOp,
  moveMode,
  onMoveModeChange,
  camDraftRef,
  hasWebcam,
}: {
  settings: AppearanceSettings;
  onClose: () => void;
  doc: EditDoc;
  timeMs: number;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean;
  onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  hasWebcam: boolean;
}) {
  const ma = settings?.screen ?? DEFAULT_APPEARANCE.screen;

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera" lede="How the webcam moves during the recording." onClose={onClose} />

      {hasWebcam ? (
        <CameraMoveField
          doc={doc}
          timeMs={timeMs}
          applyOp={applyOp}
          moveMode={moveMode}
          onMoveModeChange={onMoveModeChange}
          camDraftRef={camDraftRef}
          staticSize={ma.cam_size}
        />
      ) : (
        <div className="e-grp">
          <p className="e-hintline">
            This recording has no webcam track, so there is nothing to move. Record with the camera on to
            keyframe it.
          </p>
        </div>
      )}

      <div className="e-grp">
        <p className="e-hintline">Size, shape, position and ring live in Layouts.</p>
      </div>
    </div>
  );
}
