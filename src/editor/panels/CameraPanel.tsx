import type { RefObject } from "react";
import { PanelHeader } from "./PanelHeader";
import { CameraRingField } from "./CameraRingField";
import { CameraMoveField } from "./CameraMoveField";
import type { AppearanceSettings, ModeAppearance } from "../../hud/settings/settings";
import type { EditDoc, EditOp } from "../../lib/edit";
import { Slider, Segmented, Disclosure } from "../controls/Controls";
import type { CamPose } from "../stage/cameraMoves";
import {
  MODE_SLIDERS,
  MODE_HAS_SHAPE,
  MODE_HAS_CORNER,
  SLIDERS,
  SHAPES,
  CORNERS,
  ASPECTS,
  pct,
  DEFAULT_APPEARANCE,
  resetCameraAppearance,
  type Knob,
  type ModeKey,
} from "../../hud/preferences/appearanceFields";

// Short segment labels with the full name on hover: BL/BR/TL/TR is what fits four-up at 320px,
// and it is the wording this control has always used.
const CORNER_OPTS = CORNERS.map(([id, label]) => ({
  value: id, label, title: id.replace("_", " ").replace(/^./, (c) => c.toUpperCase()),
}));

/** Panel flow (panel pass): what the webcam IS (shape, roundness, aspect), where it sits and how
 *  big, how it is framed (ring), then how it moves - each switch above what it enables. */
export function CameraPanel({
  settings,
  onChange,
  onClose,
  doc,
  timeMs,
  applyOp,
  moveMode,
  onMoveModeChange,
  camDraftRef,
}: {
  settings: AppearanceSettings;
  onChange: (v: AppearanceSettings) => void;
  onClose: () => void;
  doc: EditDoc;
  timeMs: number;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  moveMode: boolean;
  onMoveModeChange: (v: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
}) {
  // Webcam appearance only - layout mode is chosen on the timeline, not here. Edits the
  // picture-in-picture webcam for the default screen layout.
  const mode: ModeKey = "screen";
  const ma = settings[mode] || DEFAULT_APPEARANCE[mode];

  const set = <K extends keyof ModeAppearance>(k: K, v: ModeAppearance[K]) => {
    onChange({ ...settings, [mode]: { ...ma, [k]: v } });
  };

  // cam_size, on its own. In Move mode the size is keyframed instead, and lives in
  // `CameraMoveField` under the switch that turns that on - so it drops out here.
  const sizeKnobs = MODE_SLIDERS[mode]
    .filter((k) => k === "cam_size" && !moveMode);
  // The two nudges. Dock Location is how the webcam is placed; these only trim it a few percent
  // off the corner, so they go under the panel's one disclosure, side by side (both names are two
  // short words, which is what `.e-two` is for).
  const marginKnobs = MODE_SLIDERS[mode].filter((k) => k === "cam_margin_x" || k === "cam_margin_y");

  const knob = (k: Knob) => (
    <div className="e-field" key={k}>
      <Slider min={SLIDERS[k].min} max={SLIDERS[k].max} step={SLIDERS[k].step} value={ma[k]}
        onChange={(v) => set(k, v)} ariaLabel={SLIDERS[k].label} label={SLIDERS[k].label} formatValue={pct} />
    </div>
  );

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera" lede="Shape, size and position of the webcam."
        onReset={() => onChange(resetCameraAppearance(settings))} onClose={onClose} />

      {MODE_HAS_SHAPE[mode] && (
        <div className="e-grp">
          <span className="e-sechead">Shape</span>
          <Segmented value={ma.cam_shape} options={SHAPES.map(([value, label]) => ({ value, label }))}
            onChange={(v) => set("cam_shape", v)} ariaLabel="Webcam Shape" />
          {ma.cam_shape === "rounded" && (
            <div className="e-field">
              <Slider min={SLIDERS.cam_radius.min} max={SLIDERS.cam_radius.max} step={SLIDERS.cam_radius.step}
                value={ma.cam_radius} onChange={(v) => set("cam_radius", v)} ariaLabel="Corner Roundness"
                label="Corner Roundness" formatValue={pct} />
            </div>
          )}
          <div className="e-field">
            <span className="e-fl">Aspect</span>
            <Segmented value={ma.cam_aspect} options={ASPECTS.map(([value, label]) => ({ value, label }))}
              onChange={(v) => set("cam_aspect", v)} ariaLabel="Aspect" />
          </div>
        </div>
      )}

      <div className="e-grp">
        <span className="e-sechead">Size and position</span>
        {MODE_HAS_CORNER[mode] && (
          <div className="e-field">
            <span className="e-fl">Dock Location</span>
            <Segmented value={ma.cam_corner} options={CORNER_OPTS}
              onChange={(v) => set("cam_corner", v)} ariaLabel="Dock Location" />
          </div>
        )}
        {sizeKnobs.map(knob)}
      </div>

      <CameraMoveField doc={doc} timeMs={timeMs} applyOp={applyOp} moveMode={moveMode}
        onMoveModeChange={onMoveModeChange} camDraftRef={camDraftRef} staticSize={ma.cam_size} />

      {/* The ring and the two margin nudges: both are trim on a webcam whose shape, corner and
          size are already set above, and together they are 250px the panel does not have. */}
      <Disclosure id="camera">
        <CameraRingField ring={ma.cam_ring} onChange={(v) => set("cam_ring", v)} />
        {/* No heading over the pair: each slider's own readout row already names it. */}
        <div className="e-grp"><div className="e-two">{marginKnobs.map(knob)}</div></div>
      </Disclosure>
    </div>
  );
}
