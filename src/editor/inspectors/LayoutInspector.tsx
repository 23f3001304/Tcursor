import { IconArrowsMove } from "@tabler/icons-react";
import type { EditDoc, EditOp, LayoutSeg } from "../../lib/edit";
import type { LayoutPresetName, LayoutPresets } from "../../lib/ipc";
import { NumberField, Switch } from "../controls/Controls";
import { resolvedPanelsFor } from "../timeline/layoutTrack";
import { LayoutThumb } from "../timeline/LayoutThumb";
import { poseOfRect, setArrangementOp, VISIBLE_ALPHA, type PanelKind } from "../stage/arrange/arrangeMath";
import { CurveEditor } from "./CurveEditor";
import { Hint, InspectorHeader, InspectorShell, SegRow, Section, TimingRow, secOf, spanRange } from "./InspectorShape";

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
  // arrangement hid, is its provenance preset's own placement (a hidden panel still resolves to a
  // real rect, just at alpha 0). Turning the LAST visible panel off is DISABLED rather than
  // silently dropped: `set_arrangement` rejects a write that would blank the frame, and a rejected
  // op still resolves, so a switch that only early-returned would look like it worked.
  const locked = (k: PanelKind) => !presets || (shown(k) && !shown(k === "screen" ? "cam" : "screen"));
  const setShown = (k: PanelKind, on: boolean) => {
    if (!panels || locked(k)) return;
    void onApply(setArrangementOp(seg, panels, k, on ? poseOfRect(panels[k].rect) : null));
  };

  return (
    <InspectorShell kind="layout">
      <InspectorHeader title="Layout" range={spanRange(seg.start_ms, seg.end_ms)}
        deleteLabel="Delete layout" onClose={onClose} thumb={<LayoutThumb panels={panels} w={40} h={23} />}
        onDelete={() => { void onApply({ op: "remove_layout_seg", id: seg.id }); onClose(); }} />

      <Section title="Timing">
        <TimingRow startMs={seg.start_ms} endMs={seg.end_ms} durMs={dur}
          onStart={(start_ms) => upd({ start_ms })} onEnd={(end_ms) => upd({ end_ms })} />
        <Hint>Frames the screen and webcam from here. Drag the block to move the switch point.</Hint>
      </Section>

      <Section title="Composition" value={seg.arrangement ? `Custom, based on ${pretty(seg.layout)}` : undefined}>
        <div className="e-field">
          <span className="e-fl">Start from</span>
          <SegRow ariaLabel="Start from" thumbs onPick={(k) => void startFrom(k as LayoutPresetName)}
            options={PRESETS.map((p) => ({
              key: p.value, label: p.label, on: seg.layout === p.value, disabled: !presets,
              visual: <LayoutThumb className="e-preseg-thumb" panels={presets ? { screen: presets[p.value].screen, cam: presets[p.value].cam } : null} w={84} h={49} />,
            }))} />
        </div>
        <Hint>A preset fills in the two panel positions to drag from; the segment keeps its own arrangement afterwards.</Hint>

        {seg.arrangement && (
          <button type="button" className="e-chip e-insp-reset" title="Drop this segment's own arrangement and go back to a plain preset"
            onClick={() => void onApply({ op: "clear_arrangement", id: seg.id })}>Reset to preset</button>
        )}

        {arrangeOn
          ? <Hint>Arranging on the stage, drag a panel to move it, a corner to resize it, Alt to ignore the guides. Esc when you are done.</Hint>
          : <button type="button" className="e-ghostbtn" title="Drag the panels directly on the preview" onClick={onArrange}>
              <IconArrowsMove size={15} /> Arrange on stage</button>}

        <div className="e-field" style={{ marginTop: 12 }}>
          {(["screen", "cam"] as PanelKind[]).map((k) => (
            <div className="e-switchrow" key={k}><span>{k === "screen" ? "Show screen" : "Show webcam"}</span>
              <Switch on={shown(k)} disabled={locked(k)} onChange={(v) => setShown(k, v)}
                ariaLabel={k === "screen" ? "Show screen" : "Show webcam"}
                title={locked(k) ? "One panel has to stay visible" : undefined} /></div>
          ))}
        </div>
      </Section>

      <Section title="Transition">
        <label className="e-field">
          <span className="e-fl">Transition</span>
          <NumberField step={0.05} min={0} max={2} value={secOf(seg.transition_ms)}
            onChange={(v) => upd({ transition_ms: Math.round(v * 1000) })} />
        </label>
        <CurveEditor value={seg.easing} onChange={(easing) => upd({ easing })} />

        {/* Exit: the blend COMPLETES at end_ms, mirroring the entry, which starts at start_ms.
            0 (the default) is a hard cut. A gapless next segment's own entry wins the overlap,
            so this only takes visible effect into a gap or a hard-cutting successor. */}
        <label className="e-field" style={{ marginTop: 16 }}>
          <span className="e-fl">Exit transition</span>
          <NumberField step={0.05} min={0} max={2} value={secOf(seg.transition_out_ms)}
            onChange={(v) => upd({ transition_out_ms: Math.round(v * 1000) })} />
        </label>
        {seg.transition_out_ms > 0 && (
          <CurveEditor value={seg.easing_out} label="Exit Curve" onChange={(easing_out) => upd({ easing_out })} />
        )}
      </Section>
    </InspectorShell>
  );
}
