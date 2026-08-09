import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { EditDoc, EditOp, LayoutSeg } from "../../lib/edit";
import { NumberField, Picker } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";

// "screen" is the empty default (delete a pill / leave a gap to get screen), so it isn't an
// explicit preset choice here - only the non-screen layouts.
const PRESETS = [
  { value: "camera", label: "Camera" }, { value: "presenter", label: "Presenter" },
  { value: "screen_only", label: "Screen only" }, { value: "camera_only", label: "Camera only" },
];

/** Inspector for the selected timeline layout segment. Every control applies an
 *  `update_layout_seg` (or `remove_layout_seg`) op via `onApply`, which persists the doc and
 *  bumps the preview - so edits are reflected in the live preview immediately. Shown in the
 *  left panel in place of the tab content while a layout segment is selected. */
export function LayoutInspector({ seg, dur, onApply, onClose }: {
  seg: LayoutSeg; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_layout_seg" }>, "op" | "id">>) =>
    void onApply({ op: "update_layout_seg", id: seg.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Layout" lede="Switches the frame layout from here. Drag the block to move the switch point." closeTitle="Deselect" onClose={onClose} />

      <label className="e-field">
        <span className="e-fl">Preset</span>
        <Picker value={seg.layout} options={PRESETS} onChange={(v) => upd({ layout: v })} ariaLabel="Preset" />
      </label>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">Start</span>
          <NumberField min={0} max={sec(seg.end_ms)} value={sec(seg.start_ms)}
            onChange={(v) => upd({ start_ms: Math.round(v * 1000) })} /></label>
        <label className="e-field"><span className="e-fl">End</span>
          <NumberField min={sec(seg.start_ms)} max={sec(dur)} value={sec(seg.end_ms)}
            onChange={(v) => upd({ end_ms: Math.round(v * 1000) })} /></label>
      </div>

      <label className="e-field">
        <span className="e-fl">Transition</span>
        <NumberField step={0.05} min={0} max={2} value={sec(seg.transition_ms)}
          onChange={(v) => upd({ transition_ms: Math.round(v * 1000) })} />
      </label>

      <CurveEditor value={seg.easing} onChange={(easing) => upd({ easing })} />

      {/* Exit: the blend COMPLETES at end_ms, mirroring the entry, which starts at start_ms.
          0 (the default) is a hard cut. A gapless next segment's own entry wins the overlap,
          so this only takes visible effect into a gap or a hard-cutting successor. */}
      <div className="e-sec">
        <label className="e-field">
          <span className="e-fl">Exit transition</span>
          <NumberField step={0.05} min={0} max={2} value={sec(seg.transition_out_ms)}
            onChange={(v) => upd({ transition_out_ms: Math.round(v * 1000) })} />
        </label>
        {seg.transition_out_ms > 0 && (
          <CurveEditor value={seg.easing_out} label="Exit Curve" onChange={(easing_out) => upd({ easing_out })} />
        )}
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_layout_seg", id: seg.id }); onClose(); }}>
        <IconTrash size={15} />Delete layout
      </button>
    </div>
  );
}
