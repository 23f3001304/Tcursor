import { PanelHeader } from "./PanelHeader";
import { CameraRingField } from "./CameraRingField";
import type { AppearanceSettings, ModeAppearance } from "../../hud/settings/settings";
import type { EditDoc, EditOp } from "../../lib/edit";
import { Slider, Picker, Switch } from "../controls/Controls";
import type { RefObject } from "react";
import { camKfRange, camMoveAt, type CamPose } from "../stage/cameraMoves";
import { camKeyframeAt, commitCamKeyframe } from "../stage/camKeyframeAt";
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
  type ModeKey,
} from "../../hud/preferences/appearanceFields";

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
    onChange({
      ...settings,
      [mode]: { ...ma, [k]: v },
    });
  };

  // In Move mode, "Webcam size" writes to the camera_moves keyframe at the playhead (creating
  // one if none is within the snap window) instead of the static appearance size - the static
  // cam_size applies wherever the keyframes don't own the frame (outside their span, or an
  // empty track), which is exactly where `camMoveAt` is null.
  const kfPose = camMoveAt(doc.camera_moves, timeMs);
  const kfSize = kfPose?.size ?? ma.cam_size;
  // Seed for a keyframe ADDED outside the span: the nearest end of the track (sampled at the
  // clamped time), so "Add keyframe at playhead" out there continues where the track left off
  // instead of jumping to frame-centre. The live layout pose isn't available in this panel.
  const kfRange = camKfRange(doc.camera_moves);
  const nearestPose = kfRange ? camMoveAt(doc.camera_moves, Math.min(Math.max(timeMs, kfRange[0]), kfRange[1])) : null;
  const setKfSize = (v: number) => {
    const d = camDraftRef.current;
    void commitCamKeyframe(doc.camera_moves, timeMs, d ? { x: d.x, y: d.y, size: v } : { size: v },
      { x: d?.x ?? nearestPose?.x ?? 0.5, y: d?.y ?? nearestPose?.y ?? 0.5, size: v }, applyOp);
  };
  // Save the current drafted (dragged) pose - or the sampled pose if nothing was dragged - as a
  // keyframe at the playhead. This is the ONLY thing that commits; a bare drag never does.
  const addKeyframeHere = () => {
    const p = camDraftRef.current ?? kfPose ?? nearestPose ?? { x: 0.5, y: 0.5, size: ma.cam_size };
    void commitCamKeyframe(doc.camera_moves, timeMs, { x: p.x, y: p.y, size: p.size }, p, applyOp);
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera" lede="Size, shape, and position of the picture-in-picture webcam."
        onReset={() => onChange(resetCameraAppearance(settings))} onClose={onClose} />

      <div className="e-field">
        <div className="e-switchrow">
          <span>Move in preview</span>
          <Switch on={moveMode} onChange={onMoveModeChange} />
        </div>
      </div>

      {moveMode ? (
        <>
          <p className="e-lede">Drag the webcam in the preview to reposition it freely - nothing is saved until you press the button below, and moving the playhead discards an un-saved drag.</p>

          <div className="e-field" style={{ marginBottom: 0 }}>
            <span className="e-fl">Webcam size <b>{pct(kfSize)}</b></span>
            <Slider min={SLIDERS.cam_size.min} max={SLIDERS.cam_size.max} step={SLIDERS.cam_size.step}
              value={kfSize} onChange={setKfSize} ariaLabel="Webcam size" />
          </div>

          <button type="button" className="e-ghostbtn" style={{ marginTop: 16 }} onClick={addKeyframeHere}>
            {camKeyframeAt(doc.camera_moves, timeMs) ? "Update keyframe at playhead" : "Add keyframe at playhead"}
          </button>
        </>
      ) : (
        // Static size + dock-margin sliders (position is via drag/keyframes in Move mode instead).
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {MODE_SLIDERS[mode].filter((k) => k.startsWith("cam_")).map((k) => {
            const spec = SLIDERS[k];
            return (
              <div className="e-field" key={k} style={{ marginBottom: 0 }}>
                <span className="e-fl">{spec.label} <b>{pct(ma[k])}</b></span>
                <Slider min={spec.min} max={spec.max} step={spec.step} value={ma[k]} onChange={(v) => set(k, v)} ariaLabel={spec.label} />
              </div>
            );
          })}
        </div>
      )}

      {/* Shape / roundness / aspect / dock / ring are STATIC (non-keyframed) appearance - shown in
          BOTH modes so the webcam's look is freely editable while its position is also keyframed. */}
      {MODE_HAS_SHAPE[mode] && (
        <div className="e-field" style={{ marginTop: 16 }}>
          <span className="e-fl">Webcam Shape</span>
          <div className="e-seg">
            {SHAPES.map(([shapeId, label]) => (
              <button key={shapeId} type="button" className={ma.cam_shape === shapeId ? "on" : ""}
                onClick={() => set("cam_shape", shapeId)}>{label}</button>
            ))}
          </div>
        </div>
      )}
      {MODE_HAS_SHAPE[mode] && ma.cam_shape === "rounded" && (
        <div className="e-field">
          <span className="e-fl">Corner Roundness <b>{pct(ma.cam_radius)}</b></span>
          <Slider min={SLIDERS.cam_radius.min} max={SLIDERS.cam_radius.max} step={SLIDERS.cam_radius.step}
            value={ma.cam_radius} onChange={(v) => set("cam_radius", v)} ariaLabel="Corner Roundness" />
        </div>
      )}
      <div className="e-field" style={{ marginTop: 16 }}>
        <span className="e-fl">Aspect</span>
        <Picker value={ma.cam_aspect} options={ASPECTS.map(([id, label]) => ({ value: id, label }))}
          onChange={(v) => set("cam_aspect", v)} ariaLabel="Aspect" />
      </div>
      {MODE_HAS_CORNER[mode] && (
        <div className="e-field">
          <span className="e-fl">Dock Location</span>
          <Picker value={ma.cam_corner} options={CORNERS.map(([id, label]) => ({ value: id, label: label.replace("_", " ") }))}
            onChange={(v) => set("cam_corner", v)} ariaLabel="Dock Location" />
        </div>
      )}
      <CameraRingField ring={ma.cam_ring} onChange={(v) => set("cam_ring", v)} />
    </div>
  );
}
