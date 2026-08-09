import { IconTrash } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import { NumberField } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";
import type { CameraMove, EditDoc, EditOp } from "../../lib/edit";

/** Inspector for the selected camera-move keyframe (a single-point `t_ms/x/y/size` entry on
 *  the Camera lane, not a region). Every field applies an `update_camera_move` op via
 *  `onApply`, mirroring ZoomInspector; shown in the left panel in place of the tab content
 *  while a camera-move keyframe is selected. Uses `NumberField` (the same clamped +/- stepper
 *  ZoomInspector's fields use) rather than a raw `<input type="number">` - a raw input's
 *  `onChange` fires on every keystroke, including a momentarily-cleared field, and `Number("")`
 *  is `0`; that briefly collapsed the PiP to size 0 mid-edit and could send a negative value the
 *  Rust side's u32 deserialization rejected (silently, via a `catch{}` in `applyOp`).
 *  `NumberField` has no such intermediate state - it only ever steps by a clamped amount. */
export function CameraMoveInspector({ move, dur, onApply, onClose }: {
  move: CameraMove; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_camera_move" }>, "op" | "id">>) =>
    void onApply({ op: "update_camera_move", id: move.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Camera Move" lede="Edits preview live. Drag the diamond on the timeline to retime it." closeTitle="Deselect" onClose={onClose} />

      <label className="e-field"><span className="e-fl">Time</span>
        <NumberField min={0} max={sec(dur)} step={0.05} value={sec(move.t_ms)}
          onChange={(v) => upd({ t_ms: Math.round(v * 1000) })} />
      </label>

      <div className="e-field2">
        <label className="e-field"><span className="e-fl">X</span>
          <NumberField min={0} max={1} step={0.01} unit="" value={move.x}
            onChange={(v) => upd({ x: v })} /></label>
        <label className="e-field"><span className="e-fl">Y</span>
          <NumberField min={0} max={1} step={0.01} unit="" value={move.y}
            onChange={(v) => upd({ y: v })} /></label>
      </div>

      <label className="e-field"><span className="e-fl">Size</span>
        <NumberField min={0} max={1} step={0.01} unit="" value={move.size}
          onChange={(v) => upd({ size: v })} />
      </label>

      <CurveEditor value={move.easing} onChange={(easing) => upd({ easing })} />

      <button className="e-del" onClick={() => { void onApply({ op: "remove_camera_move", id: move.id }); onClose(); }}>
        <IconTrash size={15} />Delete keyframe
      </button>
    </div>
  );
}
