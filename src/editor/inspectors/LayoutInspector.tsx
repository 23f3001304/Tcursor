import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { EditDoc, EditOp, LayoutSeg } from "../../lib/edit";
import { NumberField, Picker } from "../controls/Controls";

// "screen" is the empty default (delete a pill / leave a gap to get screen), so it isn't an
// explicit preset choice here - only the non-screen layouts.
const PRESETS = [
  { value: "camera", label: "Camera" }, { value: "presenter", label: "Presenter" },
  { value: "screen_only", label: "Screen only" }, { value: "camera_only", label: "Camera only" },
];
const CURVES = [
  { name: "Linear", key: "linear", path: "M 0 100 L 100 0" },
  { name: "Smooth", key: "smooth", path: "M 0 100 C 35 100, 65 0, 100 0" },
  { name: "Spring", key: "spring", path: "M 0 100 C 25 100, 45 -25, 75 -25 C 85 -25, 90 0, 100 0" },
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
        <Picker value={seg.layout} options={PRESETS} onChange={(v) => upd({ layout: v })} />
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

      <div className="e-field">
        <span className="e-fl">Transition Curve</span>
        <div className="e-curve-pick">
          {CURVES.map((c) => (
            <button key={c.name} className={`e-curve-card ${seg.easing === c.key ? "on" : ""}`} onClick={() => upd({ easing: c.key })}>
              <svg viewBox="0 -30 100 160" className="e-curve-svg">
                <line x1="0" y1="0" x2="100" y2="0" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <line x1="0" y1="100" x2="100" y2="100" stroke="var(--e-border2)" strokeDasharray="3 3" />
                <path d={c.path} fill="none" stroke={seg.easing === c.key ? "var(--e-fg)" : "var(--e-mut)"} strokeWidth="6" strokeLinecap="round" />
              </svg>
              <span>{c.name}</span>
            </button>
          ))}
        </div>
      </div>

      <button className="e-del" onClick={() => { void onApply({ op: "remove_layout_seg", id: seg.id }); onClose(); }}>
        <IconTrash size={15} />Delete layout
      </button>
    </div>
  );
}
