import type { RefObject } from "react";
import type { EditDoc, EditOp } from "../../lib/edit";
import { Slider, Switch } from "../controls/Controls";
import { camKfRange, camMoveAt, type CamPose } from "../stage/cameraMoves";
import { camKeyframeAt, commitCamKeyframe } from "../stage/camKeyframeAt";
import { SLIDERS, pct } from "../../hud/preferences/appearanceFields";

/** The Camera panel's "how it moves" group: the Move-in-preview switch and, directly under it,
 *  the only two controls that switch enables - the keyframed webcam size at the playhead, and the
 *  button that commits a drag as a keyframe. Split out of `CameraPanel.tsx` in the panel pass so
 *  that file reads as the panel's flow and this file holds the keyframe arithmetic. */
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
  // In Move mode, "Webcam size" writes to the camera_moves keyframe at the playhead (creating one
  // if none is within the snap window) instead of the static appearance size - the static cam_size
  // applies wherever the keyframes don't own the frame (outside their span, or an empty track),
  // which is exactly where `camMoveAt` is null.
  const kfPose = camMoveAt(doc.camera_moves, timeMs);
  const kfSize = kfPose?.size ?? staticSize;
  // Seed for a keyframe ADDED outside the span: the nearest end of the track (sampled at the
  // clamped time), so "Add keyframe at playhead" out there continues where the track left off
  // instead of jumping to frame-centre. The live layout pose isn't available in this panel.
  const kfRange = camKfRange(doc.camera_moves);
  const nearestPose = kfRange ? camMoveAt(doc.camera_moves, Math.min(Math.max(timeMs, kfRange[0]), kfRange[1])) : null;
  // Deliberately does NOT clear `camDraftRef` (review round 2, Important - unlike
  // `addKeyframeHere` below): this fires on every slider tick while dragging "Webcam size", and
  // reads `camDraftRef.current` each time to preserve a PENDING drag's x/y across the whole slider
  // gesture. Clearing it after the first tick would drop that x/y (falling back to `nearestPose`/
  // 0.5 on the very next tick, mid-slider-drag) - worse than the known, narrower gap this leaves:
  // `camDraftRef.current`'s `size` can go stale relative to what a slider commit just wrote, so a
  // PiP drag started right after (`CamDragHandle.tsx`'s `onHandlePointerDown`) seeds `start.size`
  // from that stale value instead of the size just committed here. Pre-existing, not introduced by
  // M6's fix; not a one-liner to close without the regression above, so left as-is.
  const setKfSize = (v: number) => {
    const d = camDraftRef.current;
    void commitCamKeyframe(doc.camera_moves, timeMs, d ? { x: d.x, y: d.y, size: v } : { size: v },
      { x: d?.x ?? nearestPose?.x ?? 0.5, y: d?.y ?? nearestPose?.y ?? 0.5, size: v }, applyOp);
  };
  // Save the current drafted (dragged) pose - or the sampled pose if nothing was dragged - as a
  // keyframe at the playhead. This is the ONLY thing that commits; a bare drag never does. Clears
  // camDraftRef itself once used (M6, review round 1 Important 3) - the draft's whole point is
  // to survive playback ticks until this explicit action consumes it; leaving it behind afterward
  // would let a STALE pose silently get reused as the seed for the NEXT drag/keyframe instead.
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
              value={kfSize} onChange={setKfSize} ariaLabel="Webcam size" label="Webcam size" formatValue={pct} />
          </div>
          <button type="button" className="e-ghostbtn" onClick={addKeyframeHere}>
            {camKeyframeAt(doc.camera_moves, timeMs) ? "Update keyframe at playhead" : "Add keyframe at playhead"}
          </button>
        </>
      )}
    </div>
  );
}
