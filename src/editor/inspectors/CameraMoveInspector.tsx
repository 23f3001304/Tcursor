import { PanelHeader } from "../panels/PanelHeader";
import { NumberField } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";
import { Hint, InspectorShell, RemoveButton, Section, clock, secOf } from "./InspectorShape";
import type { CameraMove, EditDoc, EditOp } from "../../lib/edit";

/** Inspector for the selected camera-move keyframe (a single-point `t_ms/x/y/size` entry on the
 *  Camera lane, not a region), so its Timing section is one Time field rather than a span.
 *
 *  Uses `NumberField` (the same clamped +/- stepper every other inspector's fields use) rather
 *  than a raw `<input type="number">` - a raw input's `onChange` fires on every keystroke,
 *  including a momentarily-cleared field, and `Number("")` is `0`; that briefly collapsed the PiP
 *  to size 0 mid-edit and could send a negative value the Rust side's u32 deserialization rejected
 *  (silently, via a `catch{}` in `applyOp`). */
export function CameraMoveInspector({ move, dur, onApply, onClose }: {
  move: CameraMove; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_camera_move" }>, "op" | "id">>) =>
    void onApply({ op: "update_camera_move", id: move.id, ...patch });

  return (
    <InspectorShell kind="cam">
      <PanelHeader title="Camera Move" lede={`Keyframe at ${clock(move.t_ms)}`} closeTitle="Deselect" onClose={onClose} />

      <Section title="Timing">
        <label className="e-field"><span className="e-fl">Time</span>
          <NumberField min={0} max={secOf(dur)} step={0.05} value={secOf(move.t_ms)}
            onChange={(v) => upd({ t_ms: Math.round(v * 1000) })} />
        </label>
        <Hint>Drag the diamond on the timeline to retime it.</Hint>
      </Section>

      <Section title="Placement">
        <div className="e-field2">
          <label className="e-field"><span className="e-fl">X</span>
            <NumberField min={0} max={1} step={0.01} unit="" value={move.x} onChange={(x) => upd({ x })} /></label>
          <label className="e-field"><span className="e-fl">Y</span>
            <NumberField min={0} max={1} step={0.01} unit="" value={move.y} onChange={(y) => upd({ y })} /></label>
        </div>
        <label className="e-field"><span className="e-fl">Size</span>
          <NumberField min={0} max={1} step={0.01} unit="" value={move.size} onChange={(size) => upd({ size })} />
        </label>
        <Hint>Edits preview live.</Hint>
      </Section>

      <Section title="Transition">
        <CurveEditor value={move.easing} onChange={(easing) => upd({ easing })} />
      </Section>

      <RemoveButton label="Delete keyframe" onClick={() => { void onApply({ op: "remove_camera_move", id: move.id }); onClose(); }} />
    </InspectorShell>
  );
}
