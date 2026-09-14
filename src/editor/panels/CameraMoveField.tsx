import type { RefObject } from "react";
import type { CamMoveShape, EditDoc, EditOp } from "../../lib/edit";
import { Slider, Switch } from "../controls/Controls";
import { CamShapeField } from "./CamShapeField";
import { camKfRange, camMoveAt, type CamPose } from "../stage/cameraMoves";
import { camKeyframeAt, commitCamKeyframe, type CamKfPatch } from "../stage/camKeyframeAt";
import { SLIDERS, pct } from "../../hud/preferences/appearanceFields";

/** The Camera panel's "how it moves" group: the Move-in-preview switch and, directly under it,
 *  the controls that switch enables - the keyframed webcam size and shape at the playhead, and
 *  the button that commits a drag as a keyframe. Split out of `CameraPanel.tsx` in the panel pass
 *  so that file reads as the panel's flow and this file holds the keyframe arithmetic. */
export function CameraMoveField({ doc, timeMs, applyOp, moveMode, onMoveModeChange, camDraftRef, staticSize }: {
  doc: EditDoc;
  timeMs: number;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean;
  onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  /** The non-keyframed `cam_size`, which applies wherever the keyframes do not own the frame. */
  staticSize: number;
}) {
  // In Move mode, "Webcam size" and "Shape" write to the camera_moves keyframe at the playhead
  // (creating one if none is within the snap window) instead of the static appearance - the
  // static cam_size applies wherever the keyframes don't own the frame (outside their span, or an
  // empty track), which is exactly where `camMoveAt` is null.
  const kfPose = camMoveAt(doc.camera_moves, timeMs);
  const kfSize = kfPose?.size ?? staticSize;
  const kfHere = camKeyframeAt(doc.camera_moves, timeMs);
  const kfShape: CamMoveShape = kfHere?.shape ?? "layout";
  // Seed for a keyframe ADDED outside the span: the nearest end of the track (sampled at the
  // clamped time), so "Add keyframe at playhead" out there continues where the track left off
  // instead of jumping to frame-centre. The live layout pose isn't available in this panel.
  const kfRange = camKfRange(doc.camera_moves);
  const nearestPose = kfRange ? camMoveAt(doc.camera_moves, Math.min(Math.max(timeMs, kfRange[0]), kfRange[1])) : null;
  // A field commit while a drag is PENDING keeps the drag: the draft's x/y ride along in the
  // keyframe, and the draft itself is updated to what was just committed (the composite draws
  // `camDraftRef` in preference to the keyframe, so a draft left holding the old size would keep
  // the PiP at that size while the slider says otherwise - the bug that made the slider look dead
  // after a drag). Only "Add/Update keyframe" below ever clears the draft.
  const commit = (patch: CamKfPatch) => {
    const d = camDraftRef.current;
    if (d) camDraftRef.current = { ...d, size: patch.size ?? d.size };
    void commitCamKeyframe(doc.camera_moves, timeMs, d ? { x: d.x, y: d.y, ...patch } : patch,
      { x: d?.x ?? nearestPose?.x ?? 0.5, y: d?.y ?? nearestPose?.y ?? 0.5, size: d?.size ?? kfSize }, applyOp);
  };
  // Save the current drafted (dragged) pose - or the sampled pose if nothing was dragged - as a
  // keyframe at the playhead. This is the ONLY thing that commits a drag; a bare drag never does.
  // Clears camDraftRef itself once used (M6, review round 1 Important 3) - the draft's whole point
  // is to survive playback ticks until this explicit action consumes it; leaving it behind
  // afterward would let a STALE pose silently get reused as the seed for the NEXT drag/keyframe.
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
          <p className="e-hintline">Drag the webcam in the preview to reposition it freely. Nothing is saved until you press the button below, and moving the playhead discards an un-saved drag.</p>
          <div className="e-field">
            <Slider min={SLIDERS.cam_size.min} max={SLIDERS.cam_size.max} step={SLIDERS.cam_size.step}
              value={kfSize} onChange={(size) => commit({ size })} ariaLabel="Webcam size" label="Webcam size" formatValue={pct} />
          </div>
          <CamShapeField shape={kfShape} roundness={kfHere?.roundness ?? 0.12}
            onShape={(shape) => commit({ shape })} onRoundness={(roundness) => commit({ roundness })} />
          <button type="button" className="e-ghostbtn" onClick={addKeyframeHere}>
            {kfHere ? "Update keyframe at playhead" : "Add keyframe at playhead"}
          </button>
        </>
      )}
    </div>
  );
}
