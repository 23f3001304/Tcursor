import type { RefObject } from "react";
import type { CamMoveShape, EditDoc, EditOp } from "../../../shared/edit";
import { Slider, Switch } from "../../controls/Controls";
import { CamShapeField } from "./CamShapeField";
import { camKfRange, camMoveAt, shapeRound, type CamPose } from "../../stage/camera/cameraMoves";
import { camKeyframeAt, commitCamKeyframe, type CamKfPatch } from "../../stage/camera/camKeyframeAt";
import { SLIDERS, pct } from "../../../hud/preferences/appearanceFields";

export function CameraMoveField({
  doc,
  timeMs,
  applyOp,
  moveMode,
  onMoveModeChange,
  camDraftRef,
  staticSize,
}: {
  doc: EditDoc;
  timeMs: number;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean;
  onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  staticSize: number;
}) {
  const kfPose = camMoveAt(doc.camera_moves, timeMs);
  const kfSize = kfPose?.size ?? staticSize;
  const kfHere = camKeyframeAt(doc.camera_moves, timeMs);
  const kfShape: CamMoveShape = kfHere?.shape ?? "layout";
  const kfRange = camKfRange(doc.camera_moves);
  const nearestPose = kfRange
    ? camMoveAt(doc.camera_moves, Math.min(Math.max(timeMs, kfRange[0]), kfRange[1]))
    : null;
  const commit = (patch: CamKfPatch) => {
    const d = camDraftRef.current;
    if (d) {
      const shape = patch.shape ?? kfShape,
        roundness = patch.roundness ?? kfHere?.roundness ?? 0.12;
      camDraftRef.current = {
        ...d,
        size: patch.size ?? d.size,
        round: shapeRound(shape, roundness) ?? undefined,
      };
    }
    void commitCamKeyframe(
      doc.camera_moves,
      timeMs,
      d ? { x: d.x, y: d.y, ...patch } : patch,
      { x: d?.x ?? nearestPose?.x ?? 0.5, y: d?.y ?? nearestPose?.y ?? 0.5, size: d?.size ?? kfSize },
      applyOp,
    );
  };
  const addKeyframeHere = () => {
    const p = camDraftRef.current ?? kfPose ?? nearestPose ?? { x: 0.5, y: 0.5, size: staticSize };
    camDraftRef.current = null;
    void commitCamKeyframe(doc.camera_moves, timeMs, { x: p.x, y: p.y, size: p.size }, p, applyOp);
  };

  return (
    <div className="e-grp">
      <span className="e-sechead">Movement</span>
      <div className="e-switchrow">
        <span>Move in preview</span>
        <Switch on={moveMode} onChange={onMoveModeChange} ariaLabel="Move in preview" />
      </div>
      {moveMode && (
        <>
          <p className="e-hintline">
            Drag the webcam in the preview to reposition it freely. Nothing is saved until you press the
            button below, and moving the playhead discards an un-saved drag.
          </p>
          <div className="e-field">
            <Slider
              min={SLIDERS.cam_size.min}
              max={SLIDERS.cam_size.max}
              step={SLIDERS.cam_size.step}
              value={kfSize}
              onChange={(size) => commit({ size })}
              ariaLabel="Webcam size"
              label="Webcam size"
              formatValue={pct}
            />
          </div>
          <CamShapeField
            shape={kfShape}
            roundness={kfHere?.roundness ?? 0.12}
            onShape={(shape) => commit({ shape })}
            onRoundness={(roundness) => commit({ roundness })}
          />
          <button type="button" className="e-ghostbtn" onClick={addKeyframeHere}>
            {kfHere ? "Update keyframe at playhead" : "Add keyframe at playhead"}
          </button>
        </>
      )}
    </div>
  );
}
