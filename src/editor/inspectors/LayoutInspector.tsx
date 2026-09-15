import { IconArrowsMove } from "@tabler/icons-react";
import type { EditDoc, EditOp, LayoutSeg } from "../../shared/edit";
import type { LayoutPresetName, LayoutPresets } from "../../shared/ipc";
import { NumberField, Switch } from "../controls/Controls";
import { resolvedPanelsFor } from "../timeline/model/layoutTrack";
import { LayoutThumb } from "../timeline/lanes/LayoutLane";
import { poseOfRect, setArrangementOp, VISIBLE_ALPHA, type PanelKind } from "../stage/arrange/arrangeMath";
import { MotionField, motionReadout } from "../motion/MotionField";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, spanRange } from "./InspectorShape";
import { SegRow, TimingRow } from "./InspectorRows";
import { LAYOUT_PRESETS, layoutGraphInput, prettyLayout } from "./layoutInspectorModel";

export function LayoutInspector({
  seg,
  segs,
  dur,
  presets,
  arrangeOn,
  onApply,
  onArrange,
  onClose,
}: {
  seg: LayoutSeg;
  segs: LayoutSeg[];
  dur: number;
  presets: LayoutPresets | null;
  arrangeOn: boolean;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onArrange: () => void;
  onClose: () => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_layout_seg" }>, "op" | "id">>) =>
    void onApply({ op: "update_layout_seg", id: seg.id, ...patch });
  const panels = presets ? resolvedPanelsFor(seg, presets) : null;
  const shown = (k: PanelKind) => !panels || panels[k].alpha > VISIBLE_ALPHA;
  const startFrom = async (v: LayoutPresetName) => {
    await onApply({ op: "update_layout_seg", id: seg.id, layout: v });
    const a = presets?.[v].arrangement;
    if (a) await onApply({ op: "set_arrangement", id: seg.id, screen: a.screen, cam: a.cam });
  };
  const locked = (k: PanelKind) => !presets || (shown(k) && !shown(k === "screen" ? "cam" : "screen"));
  const setShown = (k: PanelKind, on: boolean) => {
    if (!panels || locked(k)) return;
    void onApply(setArrangementOp(seg, panels, k, on ? poseOfRect(panels[k].rect) : null));
  };

  return (
    <InspectorShell kind="layout">
      <InspectorHeader
        title="Layout"
        range={spanRange(seg.start_ms, seg.end_ms)}
        deleteLabel="Delete layout"
        onClose={onClose}
        thumb={<LayoutThumb panels={panels} w={40} h={23} />}
        onDelete={() => {
          void onApply({ op: "remove_layout_seg", id: seg.id });
          onClose();
        }}
      />

      <Section title="Timing">
        <TimingRow
          startMs={seg.start_ms}
          endMs={seg.end_ms}
          durMs={dur}
          onStart={(start_ms) => upd({ start_ms })}
          onEnd={(end_ms) => upd({ end_ms })}
        />
        <Hint>Frames the screen and webcam from here. Drag the block to move the switch point.</Hint>
      </Section>

      <Section
        title="Composition"
        value={seg.arrangement ? `Custom, based on ${prettyLayout(seg.layout)}` : undefined}
      >
        <div className="e-field">
          <span className="e-fl">Start from</span>
          <SegRow
            ariaLabel="Start from"
            thumbs
            onPick={(k) => void startFrom(k as LayoutPresetName)}
            options={LAYOUT_PRESETS.map((p) => ({
              key: p.value,
              label: p.label,
              on: seg.layout === p.value,
              disabled: !presets,
              visual: (
                <LayoutThumb
                  className="e-preseg-thumb"
                  panels={presets ? { screen: presets[p.value].screen, cam: presets[p.value].cam } : null}
                  w={84}
                  h={49}
                />
              ),
            }))}
          />
        </div>
        <Hint>
          A preset fills in the two panel positions to drag from; the segment keeps its own arrangement
          afterwards.
        </Hint>

        {seg.arrangement && (
          <button
            type="button"
            className="e-chip e-insp-reset"
            title="Drop this segment's own arrangement and go back to a plain preset"
            onClick={() => void onApply({ op: "clear_arrangement", id: seg.id })}
          >
            Reset to preset
          </button>
        )}

        {arrangeOn ? (
          <Hint>
            Arranging on the stage, drag a panel to move it, a corner to resize it, Alt to ignore the guides.
            Esc when you are done.
          </Hint>
        ) : (
          <button
            type="button"
            className="e-ghostbtn"
            title="Drag the panels directly on the preview"
            onClick={onArrange}
          >
            <IconArrowsMove size={15} /> Arrange on stage
          </button>
        )}

        <div className="e-field" style={{ marginTop: 12 }}>
          {(["screen", "cam"] as PanelKind[]).map((k) => (
            <div className="e-switchrow" key={k}>
              <span>{k === "screen" ? "Show screen" : "Show webcam"}</span>
              <Switch
                on={shown(k)}
                disabled={locked(k)}
                onChange={(v) => setShown(k, v)}
                ariaLabel={k === "screen" ? "Show screen" : "Show webcam"}
                title={locked(k) ? "One panel has to stay visible" : undefined}
              />
            </div>
          ))}
        </div>
      </Section>

      <Section title="Motion" value={motionReadout(seg.easing, seg.easing_out)}>
        <div className="e-field2">
          <label className="e-field">
            <span className="e-fl">Entry</span>
            <NumberField
              step={0.05}
              min={0}
              max={2}
              value={secOf(seg.transition_ms)}
              onChange={(v) => upd({ transition_ms: Math.round(v * 1000) })}
            />
          </label>
          <label className="e-field">
            <span className="e-fl">Exit</span>
            <NumberField
              step={0.05}
              min={0}
              max={2}
              value={secOf(seg.transition_out_ms)}
              onChange={(v) => upd({ transition_out_ms: Math.round(v * 1000) })}
            />
          </label>
        </div>
        <MotionField
          input={layoutGraphInput(seg, segs)}
          easing={seg.easing}
          easingOut={seg.easing_out}
          onPatch={(p) =>
            upd({
              easing: p.easing,
              easing_out: p.easing_out,
              transition_ms: p.inMs,
              transition_out_ms: p.outMs,
            })
          }
        />
      </Section>
    </InspectorShell>
  );
}
