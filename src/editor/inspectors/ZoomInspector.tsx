import { IconCrosshair, IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { CamZoomAction, EditDoc, EditOp, Zoom, ZoomTarget } from "../../lib/edit";
import { NumberField, Slider } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";

/** The two target modes the picker offers. `ZoomTarget::Fixed{x,y}` is the whole Region model -
 *  the old "Center" button was only ever `fixed{0.5,0.5}`, so a doc written by it selects Region
 *  with its reticle already at the centre and needs no migration. */
export type TargetMode = "cursor" | "region";
export const targetMode = (t: ZoomTarget): TargetMode => (t === "cursor" ? "cursor" : "region");
/** Switching TO Region keeps whatever point is already stored, so toggling Follow cursor -> Region
 *  -> Follow cursor never silently discards an aim the user placed; a zoom that has only ever
 *  followed the cursor starts at frame centre. */
export function targetForMode(mode: TargetMode, current: ZoomTarget): ZoomTarget {
  if (mode === "cursor") return "cursor";
  return typeof current === "object" ? current : { fixed: { x: 0.5, y: 0.5 } };
}

// The per-zoom webcam override (`Zoom.cam_action`, `set_zoom_cam_action` op) - `null` inherits
// the global `settings.zoom.cam_zoom_default` (see `resolvedCamDefault`, stage/camZoomAction.ts).
// `Shrink.to` mirrors the Rust `ZoomSettings::camera_shrink_min` default (0.62).
export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[] = [
  { label: "Global default", value: null },
  { label: "Stay", value: "stay" },
  { label: "Shrink", value: { shrink: { to: 0.62 } } },
  { label: "Hide", value: "hide" },
];

/** Whether `option` is the one currently in effect for `current` (`Zoom.cam_action`). The
 *  `Shrink` variant carries a `to` fraction that isn't user-editable here, so any Shrink value
 *  counts as a match - it's the only object-shaped variant. */
export function isCamActionSelected(current: CamZoomAction | null | undefined, option: CamZoomAction | null): boolean {
  if (option === null) return current == null;
  if (typeof option === "string") return current === option;
  return typeof current === "object" && current !== null && "shrink" in current;
}

const PRESETS = [
  { name: "Subtle", scale: 1.6, zoom_in_ms: 400, zoom_out_ms: 500, easing: "smooth" },
  { name: "Balanced", scale: 2.2, zoom_in_ms: 350, zoom_out_ms: 450, easing: "smooth" },
  { name: "Punchy", scale: 2.8, zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" },
];

const activePreset = (z: Zoom) => {
  const p = PRESETS.find(
    (p) =>
      Math.abs(p.scale - z.scale) < 0.05 &&
      p.zoom_in_ms === z.zoom_in_ms &&
      p.zoom_out_ms === z.zoom_out_ms &&
      p.easing === z.easing
  );
  return p ? p.name : "Custom";
};

/** Inspector for the selected timeline zoom block. Every control applies an `update_zoom`
 *  (or `remove_zoom`) op via `onApply`, which persists the doc and bumps the preview - so
 *  edits are reflected in the live preview immediately. Shown in the left panel in place
 *  of the tab content while a zoom is selected. */
export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode }: {
  zoom: Zoom; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
  aimMode: boolean; moveMode: boolean; onAimMode: (on: boolean) => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_zoom" }>, "op" | "id">>) =>
    void onApply({ op: "update_zoom", id: zoom.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const mode = targetMode(zoom.target);
  const curPreset = activePreset(zoom);

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Zoom" lede="Edits preview live. Drag the block on the timeline to move it." closeTitle="Deselect" onClose={onClose} />

      <label className="e-field">
        <span className="e-fl">Scale <b>{zoom.scale.toFixed(1)}x</b></span>
        <Slider min={1} max={4} step={0.1} value={zoom.scale}
          onChange={(v) => upd({ scale: v })} accentColor="var(--e-zoom)" ariaLabel="Scale" />
      </label>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Start</span>
          <NumberField min={0} max={sec(zoom.end_ms)} value={sec(zoom.start_ms)}
            onChange={(v) => upd({ start_ms: Math.round(v * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">End</span>
          <NumberField min={sec(zoom.start_ms)} max={sec(dur)} value={sec(zoom.end_ms)}
            onChange={(v) => upd({ end_ms: Math.round(v * 1000) })} /></label>
      </div>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Zoom in</span>
          <NumberField step={0.05} min={0} max={sec(zoom.end_ms - zoom.start_ms)} value={sec(zoom.zoom_in_ms)}
            onChange={(v) => upd({ zoom_in_ms: Math.round(v * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">Zoom out</span>
          <NumberField step={0.05} min={0} max={sec(zoom.end_ms - zoom.start_ms)} value={sec(zoom.zoom_out_ms)}
            onChange={(v) => upd({ zoom_out_ms: Math.round(v * 1000) })} /></label>
      </div>

      <div className="e-field">
        <span className="e-fl">Feel</span>
        <div className="e-seg" style={{ flexWrap: "wrap" }}>
          {PRESETS.map((p) => (
            <button key={p.name} className={curPreset === p.name ? "on" : ""} onClick={() => upd({ scale: p.scale, zoom_in_ms: p.zoom_in_ms, zoom_out_ms: p.zoom_out_ms, easing: p.easing })}>
              {p.name}
            </button>
          ))}
        </div>
      </div>

      <CurveEditor value={zoom.easing} onChange={(easing) => upd({ easing })} />

      <div className="e-field">
        <span className="e-fl">Target</span>
        <div className="e-seg">
          <button className={mode === "cursor" ? "on" : ""}
            onClick={() => { onAimMode(false); upd({ target: targetForMode("cursor", zoom.target) }); }}>Follow cursor</button>
          <button className={mode === "region" ? "on" : ""}
            onClick={() => upd({ target: targetForMode("region", zoom.target) })}>Region</button>
        </div>
        {mode === "region" && (
          <button type="button" className={`e-aimbtn${aimMode ? " on" : ""}`} disabled={moveMode}
            title={moveMode ? "Turn off Move in preview first" : "Click or drag the preview to place this zoom's aim point"}
            onClick={() => onAimMode(!aimMode)}>
            <IconCrosshair size={14} />{aimMode ? "Aiming - click the preview" : "Aim on stage"}
          </button>
        )}
      </div>

      <div className="e-field">
        <span className="e-fl">Webcam during zoom</span>
        <div className="e-seg" style={{ flexWrap: "wrap" }}>
          {CAM_ACTION_OPTIONS.map((opt) => (
            <button key={opt.label} type="button" className={isCamActionSelected(zoom.cam_action, opt.value) ? "on" : ""}
              onClick={() => void onApply({ op: "set_zoom_cam_action", id: zoom.id, action: opt.value })}>
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_zoom", id: zoom.id }); onClose(); }}>
        <IconTrash size={15} /> Delete zoom
      </button>
    </div>
  );
}
