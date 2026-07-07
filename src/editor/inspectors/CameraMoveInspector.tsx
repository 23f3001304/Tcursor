import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import { CAM_CURVES } from "./curves";
import type { CameraMove, EditDoc, EditOp } from "../../lib/edit";

const numStyle: React.CSSProperties = {
  width: "100%", height: 36, boxSizing: "border-box", padding: "0 10px",
  background: "var(--e-soft)", border: "1px solid var(--e-border)", borderRadius: "var(--e-r)",
  color: "var(--e-fg)", fontSize: 13, fontWeight: 600, fontVariantNumeric: "tabular-nums",
};

/** Inspector for the selected camera-move keyframe (a single-point `t_ms/x/y/size` entry on
 *  the Camera lane, not a region). Every field applies an `update_camera_move` op via
 *  `onApply`, mirroring ZoomInspector; shown in the left panel in place of the tab content
 *  while a camera-move keyframe is selected. */
export function CameraMoveInspector({ move, dur, onApply, onClose }: {
  move: CameraMove; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_camera_move" }>, "op" | "id">>) =>
    void onApply({ op: "update_camera_move", id: move.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const frac = (v: number) => Math.max(0, Math.min(1, v));

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera Move" lede="Edits preview live. Drag the diamond on the timeline to retime it." closeTitle="Deselect" onClose={onClose} />

      <label className="e-field"><span className="e-fl">Time</span>
        <input type="number" style={numStyle} min={0} max={sec(dur)} step={0.05} value={sec(move.t_ms)}
          onChange={(e) => upd({ t_ms: Math.round(Number(e.target.value) * 1000) })} />
      </label>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">X</span>
          <input type="number" style={numStyle} min={0} max={1} step={0.01} value={move.x}
            onChange={(e) => upd({ x: frac(Number(e.target.value)) })} /></label>
        <label className="e-field"><span className="e-fl">Y</span>
          <input type="number" style={numStyle} min={0} max={1} step={0.01} value={move.y}
            onChange={(e) => upd({ y: frac(Number(e.target.value)) })} /></label>
      </div>

      <label className="e-field"><span className="e-fl">Size</span>
        <input type="number" style={numStyle} min={0} max={1} step={0.01} value={move.size}
          onChange={(e) => upd({ size: frac(Number(e.target.value)) })} />
      </label>

      <div className="e-field">
        <span className="e-fl">Transition Curve</span>
        <div className="e-curve-pick">
          {CAM_CURVES.map((c) => (
            <button key={c.key} className={`e-curve-card ${move.easing === c.key ? "on" : ""}`} onClick={() => upd({ easing: c.key })}>
              <svg viewBox="0 -30 100 160" className="e-curve-svg">
                <line x1="0" y1="0" x2="100" y2="0" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <line x1="0" y1="100" x2="100" y2="100" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <path d={c.path} fill="none" stroke={move.easing === c.key ? "var(--e-fg)" : "var(--e-mut)"} strokeWidth="6" strokeLinecap="round" />
              </svg>
              <span>{c.name}</span>
            </button>
          ))}
        </div>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_camera_move", id: move.id }); onClose(); }}>
        <IconTrash size={15} />Delete keyframe
      </button>
    </div>
  );
}
