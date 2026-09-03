import { IconTrash, IconArrowsMove } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { EditDoc, EditOp, LayoutSeg } from "../../lib/edit";
import type { LayoutPresetName, LayoutPresets } from "../../lib/ipc";
import { NumberField, Switch } from "../controls/Controls";
import { resolvedPanelsFor } from "../timeline/layoutTrack";
import { poseOfRect, setArrangementOp, VISIBLE_ALPHA, type PanelKind } from "../stage/arrange/arrangeMath";
import { CurveEditor } from "./CurveEditor";

// "screen" is the empty default (delete a pill / leave a gap to get screen) AND the one layout the
// timeline hides as a pill, so starting from it would make the segment being edited vanish from
// the track - only the four explicit layouts are offered.
const PRESETS: { value: LayoutPresetName; label: string }[] = [
  { value: "camera", label: "Camera" }, { value: "presenter", label: "Presenter" },
  { value: "screen_only", label: "Screen only" }, { value: "camera_only", label: "Camera only" },
];
const pretty = (v: string) => v.replace(/_/g, " ").replace(/^./, (c) => c.toUpperCase());

/** Inspector for the selected timeline layout segment. Timing/transition controls apply an
 *  `update_layout_seg` op; the arrangement controls apply `set_arrangement`/`clear_arrangement`.
 *  A preset here is a STARTING POINT: it fills the segment's two panel poses in so they can be
 *  dragged on the stage, and from then on the segment carries its own arrangement - it does not
 *  stay bound to the preset, and it is not a claim that the two render identically. */
export function LayoutInspector({ seg, dur, presets, arrangeOn, onApply, onArrange, onClose }: {
  seg: LayoutSeg; dur: number; presets: LayoutPresets | null; arrangeOn: boolean;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onArrange: () => void; onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_layout_seg" }>, "op" | "id">>) =>
    void onApply({ op: "update_layout_seg", id: seg.id, ...patch });
  const sec = (ms: number) => +(ms / 1000).toFixed(2);
  const panels = presets ? resolvedPanelsFor(seg, presets) : null;
  const shown = (k: PanelKind) => !panels || panels[k].alpha > VISIBLE_ALPHA;
  // Both ops land inside the history's 400ms coalesce window, so this is ONE undo step. The
  // `layout` name has to move too: it is what selects the appearance block the poses resolve
  // against (radius/ring/shape), so poses from one preset under another's block would not be the
  // preset the chip names.
  const startFrom = async (v: LayoutPresetName) => {
    await onApply({ op: "update_layout_seg", id: seg.id, layout: v });
    const a = presets?.[v].arrangement;
    if (a) await onApply({ op: "set_arrangement", id: seg.id, screen: a.screen, cam: a.cam });
  };
  // Re-showing a panel restores it at whatever it currently RESOLVES to - which, for one the
  // arrangement hid, is its provenance preset's own placement (L1: a hidden panel still resolves
  // to a real rect, just at alpha 0). Turning the LAST visible panel off is DISABLED rather than
  // silently dropped: `set_arrangement` rejects a write that would blank the frame, and a rejected
  // op still resolves, so a switch that only early-returned would look like it worked.
  const locked = (k: PanelKind) => !presets || (shown(k) && !shown(k === "screen" ? "cam" : "screen"));
  const setShown = (k: PanelKind, on: boolean) => {
    if (!panels || locked(k)) return;
    void onApply(setArrangementOp(seg, panels, k, on ? poseOfRect(panels[k].rect) : null));
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Layout" lede="Frames the screen and webcam from here. Drag the block to move the switch point." closeTitle="Deselect" onClose={onClose} />

      <div className="e-field">
        <span className="e-fl">Start from</span>
        <div className="e-seg" style={{ flexWrap: "wrap" }}>
          {PRESETS.map((p) => (
            <button key={p.value} type="button" className={seg.layout === p.value ? "on" : ""}
              disabled={!presets} onClick={() => void startFrom(p.value)}>{p.label}</button>
          ))}
        </div>
      </div>
      <p className="e-sec-hint">A preset fills in the two panel positions to drag from; the segment keeps its own arrangement afterwards. "Reset to preset" puts it back on a plain preset.</p>

      {arrangeOn
        ? <p className="e-sec-hint">Arranging on the stage - drag a panel to move it, a corner to resize it, Alt to ignore the guides. Esc when you are done.</p>
        : <button type="button" className="e-ghostbtn" onClick={onArrange}><IconArrowsMove size={15} /> Arrange on stage</button>}

      {seg.arrangement && (
        <div className="e-secrow" style={{ marginTop: 12 }}>
          <span className="e-fl">Custom arrangement, based on {pretty(seg.layout)}</span>
          <button type="button" className="e-chip" onClick={() => void onApply({ op: "clear_arrangement", id: seg.id })}>Reset to preset</button>
        </div>
      )}

      <div className="e-field" style={{ marginTop: 12 }}>
        {(["screen", "cam"] as PanelKind[]).map((k) => (
          <div className="e-switchrow" key={k}><span>{k === "screen" ? "Show screen" : "Show webcam"}</span>
            <Switch on={shown(k)} disabled={locked(k)} onChange={(v) => setShown(k, v)}
              ariaLabel={k === "screen" ? "Show screen" : "Show webcam"}
              title={locked(k) ? "One panel has to stay visible" : undefined} /></div>
        ))}
      </div>

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
