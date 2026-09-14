import type { CamMoveShape } from "../../lib/edit";
import { Segmented, Slider } from "../controls/Controls";
import { SLIDERS, pct } from "../../hud/preferences/appearanceFields";

/** The four shapes a camera-move keyframe can carry. "Layout" is the default and means "whatever
 *  the layout's webcam shape is" - every keyframe made before shapes existed reads as it. */
export const CAM_MOVE_SHAPES: [CamMoveShape, string][] = [["layout", "Layout"], ["circle", "Circle"], ["rounded", "Rounded"], ["rect", "Rect"]];

/** A keyframe's shape: the four-way picker and, for Rounded only, its corner roundness (the same
 *  fraction-of-short-side slider the Layouts panel uses for the static webcam). Shared by the
 *  Move-mode field (writes the keyframe at the playhead) and the keyframe inspector (writes the
 *  selected keyframe) so the two cannot offer different shapes. */
export function CamShapeField({ shape, roundness, onShape, onRoundness }: {
  shape: CamMoveShape; roundness: number;
  onShape: (v: CamMoveShape) => void; onRoundness: (v: number) => void;
}) {
  return (
    <>
      <div className="e-field">
        <span className="e-fl">Shape</span>
        <Segmented value={shape} options={CAM_MOVE_SHAPES.map(([value, label]) => ({ value, label }))}
          onChange={onShape} ariaLabel="Keyframe Shape" />
      </div>
      {shape === "rounded" && (
        <div className="e-field">
          <Slider min={SLIDERS.cam_radius.min} max={SLIDERS.cam_radius.max} step={SLIDERS.cam_radius.step}
            value={roundness} onChange={onRoundness} ariaLabel="Corner Roundness" label="Corner Roundness" formatValue={pct} />
        </div>
      )}
    </>
  );
}
