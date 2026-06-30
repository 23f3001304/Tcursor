import { IconX, IconTrash } from "@tabler/icons-react";
import type { EditDoc, EditOp, EffectRegion } from "../lib/edit";

/** Inspector for the selected effect region (spotlight). v1 edits start/end + delete; the
 *  spotlight's look comes from Settings (per-region params are a later addition). Shown in the
 *  left panel in place of the tab content while an effect region is selected. */
export function EffectInspector({ effect, dur, onApply, onClose }: {
  effect: EffectRegion; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const upd = (patch: { start_ms?: number; end_ms?: number }) =>
    void onApply({ op: "update_effect", id: effect.id, ...patch });

  return (
    <div className="e-panel e-insp">
      <div className="e-insphdr">
        <h2>Spotlight</h2>
        <button className="e-gst" title="Deselect" onClick={onClose}><IconX size={16} /></button>
      </div>
      <p className="e-lede">Dims everything but the cursor for this span. Its look comes from Settings; drag the block on the timeline to move it.</p>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Start (s)</span>
          <input type="number" step={0.1} min={0} max={sec(effect.end_ms)} value={sec(effect.start_ms)}
            onChange={(e) => upd({ start_ms: Math.round(Number(e.target.value) * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">End (s)</span>
          <input type="number" step={0.1} min={sec(effect.start_ms)} max={sec(dur)} value={sec(effect.end_ms)}
            onChange={(e) => upd({ end_ms: Math.round(Number(e.target.value) * 1000) })} /></label>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_effect", id: effect.id }); onClose(); }}>
        <IconTrash size={15} /> Delete spotlight
      </button>
    </div>
  );
}
