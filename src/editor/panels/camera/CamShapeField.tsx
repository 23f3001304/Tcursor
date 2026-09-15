import type { CamMoveShape } from "../../../shared/edit";
import { Segmented, Slider } from "../../controls/Controls";
import { SLIDERS, pct } from "../../../hud/preferences/appearanceFields";

export const CAM_MOVE_SHAPES: [CamMoveShape, string][] = [
  ["layout", "Layout"],
  ["circle", "Circle"],
  ["rounded", "Rounded"],
  ["rect", "Rect"],
];

export function CamShapeField({
  shape,
  roundness,
  onShape,
  onRoundness,
}: {
  shape: CamMoveShape;
  roundness: number;
  onShape: (v: CamMoveShape) => void;
  onRoundness: (v: number) => void;
}) {
  return (
    <>
      <div className="e-field">
        <span className="e-fl">Shape</span>
        <Segmented
          value={shape}
          options={CAM_MOVE_SHAPES.map(([value, label]) => ({ value, label }))}
          onChange={onShape}
          ariaLabel="Keyframe Shape"
        />
      </div>
      {shape === "rounded" && (
        <div className="e-field">
          <Slider
            min={SLIDERS.cam_radius.min}
            max={SLIDERS.cam_radius.max}
            step={SLIDERS.cam_radius.step}
            value={roundness}
            onChange={onRoundness}
            ariaLabel="Corner Roundness"
            label="Corner Roundness"
            formatValue={pct}
          />
        </div>
      )}
    </>
  );
}
