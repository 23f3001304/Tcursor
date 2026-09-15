import type { ModeAppearance } from "../../../hud/settings/settings";
import { Slider, Segmented } from "../../controls/Controls";
import { CameraRingField } from "../camera/CameraRingField";
import {
  MODE_SLIDERS,
  MODE_HAS_SHAPE,
  MODE_HAS_CORNER,
  SLIDERS,
  SHAPES,
  CORNERS,
  ASPECTS,
  pct,
  type Knob,
  type ModeKey,
} from "../../../hud/preferences/appearanceFields";

const CORNER_OPTS = CORNERS.map(([id, label]) => ({
  value: id,
  label,
  title: id.replace("_", " ").replace(/^./, (c) => c.toUpperCase()),
}));

const SCREEN_KNOBS: Knob[] = ["pad", "screen_size", "screen_radius"];

const CAM_KNOBS: Knob[] = ["cam_size", "cam_margin_x", "cam_margin_y"];

const has = (mode: ModeKey, k: Knob) => MODE_SLIDERS[mode].includes(k);

export function LayoutKnobs({
  mode,
  ma,
  onChange,
}: {
  mode: ModeKey;
  ma: ModeAppearance;
  onChange: (next: ModeAppearance) => void;
}) {
  const set = <K extends keyof ModeAppearance>(k: K, v: ModeAppearance[K]) => onChange({ ...ma, [k]: v });
  const knob = (k: Knob) => (
    <div className="e-field" key={k}>
      <Slider
        min={SLIDERS[k].min}
        max={SLIDERS[k].max}
        step={SLIDERS[k].step}
        value={ma[k]}
        onChange={(v) => set(k, v)}
        ariaLabel={SLIDERS[k].label}
        label={SLIDERS[k].label}
        formatValue={pct}
      />
    </div>
  );

  const screenKnobs = SCREEN_KNOBS.filter((k) => has(mode, k));
  const margins = CAM_KNOBS.filter((k) => k !== "cam_size" && has(mode, k));
  const showCam = MODE_HAS_SHAPE[mode];

  return (
    <>
      {screenKnobs.length > 0 && (
        <div className="e-grp">
          <span className="e-sechead">Screen</span>
          {screenKnobs.map(knob)}
        </div>
      )}

      {showCam && (
        <div className="e-grp">
          <span className="e-sechead">Camera</span>
          {has(mode, "cam_size") && knob("cam_size")}
          <div className="e-field">
            <span className="e-fl">Shape</span>
            <Segmented
              value={ma.cam_shape}
              options={SHAPES.map(([value, label]) => ({ value, label }))}
              onChange={(v) => set("cam_shape", v)}
              ariaLabel="Webcam Shape"
            />
          </div>
          {ma.cam_shape === "rounded" && (
            <div className="e-field">
              <Slider
                min={SLIDERS.cam_radius.min}
                max={SLIDERS.cam_radius.max}
                step={SLIDERS.cam_radius.step}
                value={ma.cam_radius}
                onChange={(v) => set("cam_radius", v)}
                ariaLabel="Corner Roundness"
                label="Corner Roundness"
                formatValue={pct}
              />
            </div>
          )}
          <div className="e-field">
            <span className="e-fl">Aspect</span>
            <Segmented
              value={ma.cam_aspect}
              options={ASPECTS.map(([value, label]) => ({ value, label }))}
              onChange={(v) => set("cam_aspect", v)}
              ariaLabel="Aspect"
            />
          </div>
          {MODE_HAS_CORNER[mode] && (
            <div className="e-field">
              <span className="e-fl">Dock Location</span>
              <Segmented
                value={ma.cam_corner}
                options={CORNER_OPTS}
                onChange={(v) => set("cam_corner", v)}
                ariaLabel="Dock Location"
              />
            </div>
          )}
          {margins.length > 0 && <div className="e-two">{margins.map(knob)}</div>}
        </div>
      )}

      {showCam && <CameraRingField ring={ma.cam_ring} onChange={(v) => set("cam_ring", v)} />}
    </>
  );
}
