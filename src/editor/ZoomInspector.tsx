import { IconX, IconTrash } from "@tabler/icons-react";
import type { EditDoc, EditOp, Zoom } from "../lib/edit";

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

  return (
    <div className="e-panel e-insp">
      <div className="e-insphdr">
        <h2>Zoom</h2>
        <button className="e-gst" title="Deselect" onClick={onClose}><IconX size={16} /></button>
      </div>
      <p className="e-lede">Edits preview live. Drag the block on the timeline to move it.</p>

      <label className="e-field">
        <span className="e-fl">Scale <b>{zoom.scale.toFixed(1)}x</b></span>
        <input type="range" min={1} max={4} step={0.1} value={zoom.scale}
          onChange={(e) => upd({ scale: Number(e.target.value) })} />
      </label>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Start (s)</span>
          <input type="number" step={0.1} min={0} max={sec(zoom.end_ms)} value={sec(zoom.start_ms)}
            onChange={(e) => upd({ start_ms: Math.round(Number(e.target.value) * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">End (s)</span>
          <input type="number" step={0.1} min={sec(zoom.start_ms)} max={sec(dur)} value={sec(zoom.end_ms)}
            onChange={(e) => upd({ end_ms: Math.round(Number(e.target.value) * 1000) })} /></label>
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
