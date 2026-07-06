import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { EditDoc, EditOp, Zoom } from "../../lib/edit";
import { NumberField, Slider } from "../controls/Controls";

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
export function ZoomInspector({ zoom, dur, onApply, onClose }: {
  zoom: Zoom; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_zoom" }>, "op" | "id">>) =>
    void onApply({ op: "update_zoom", id: zoom.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const isCursor = zoom.target === "cursor";
  const curPreset = activePreset(zoom);

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Zoom" lede="Edits preview live. Drag the block on the timeline to move it." closeTitle="Deselect" onClose={onClose} />

      <label className="e-field">
        <span className="e-fl">Scale <b>{zoom.scale.toFixed(1)}x</b></span>
        <Slider min={1} max={4} step={0.1} value={zoom.scale}
          onChange={(v) => upd({ scale: v })} accentColor="var(--e-zoom)" />
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
          <button className={curPreset === "Custom" ? "on" : ""} disabled style={{ opacity: 0.5 }}>Custom</button>
        </div>
      </div>

      <div className="e-field">
        <span className="e-fl">Transition Curve</span>
        <div className="e-curve-pick">
          {[
            { name: "Linear", key: "linear", path: "M 0 100 L 100 0" },
            { name: "Smooth", key: "smooth", path: "M 0 100 C 35 100, 65 0, 100 0" },
            { name: "Spring", key: "spring", path: "M 0 100 C 25 100, 45 -25, 75 -25 C 85 -25, 90 0, 100 0" }
          ].map((c) => (
            <button key={c.name} className={`e-curve-card ${zoom.easing === c.key ? "on" : ""}`} onClick={() => upd({ easing: c.key })}>
              <svg viewBox="0 -30 100 160" className="e-curve-svg">
                <line x1="0" y1="0" x2="100" y2="0" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <line x1="0" y1="100" x2="100" y2="100" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <path d={c.path} fill="none" stroke={zoom.easing === c.key ? "var(--e-fg)" : "var(--e-mut)"} strokeWidth="6" strokeLinecap="round" />
              </svg>
              <span>{c.name}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="e-field">
        <span className="e-fl">Target</span>
        <div className="e-seg">
          <button className={isCursor ? "on" : ""} onClick={() => upd({ target: "cursor" })}>Follow cursor</button>
          <button className={!isCursor ? "on" : ""} onClick={() => upd({ target: { fixed: { x: 0.5, y: 0.5 } } })}>Center</button>
        </div>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_zoom", id: zoom.id }); onClose(); }}>
        <IconTrash size={15} /> Delete zoom
      </button>
    </div>
  );
}
