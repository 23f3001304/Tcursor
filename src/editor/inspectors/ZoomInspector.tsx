import type { RefObject } from "react";
import { IconCrosshair } from "@tabler/icons-react";
import type { EditDoc, EditOp, Zoom } from "../../shared/edit";
import { Slider } from "../controls/Controls";
import { MotionField, motionReadout } from "../motion/MotionField";
import { Hint, InspectorHeader, InspectorShell, Section, secOf, spanRange } from "./InspectorShape";
import { SegRow, ValueRow } from "./InspectorRows";
import {
  CAM_ACTION_OPTIONS,
  durationOptions,
  isCamActionSelected,
  targetForMode,
  targetMode,
  zoomGraphInput,
  zoomScopedSeekMs,
  type TargetMode,
} from "./zoomInspectorModel";

export function ZoomInspector({
  zoom,
  zooms,
  dur,
  onApply,
  onClose,
  aimMode,
  moveMode,
  onAimMode,
  timeMsRef,
  onSeek,
}: {
  zoom: Zoom;
  zooms: Zoom[];
  dur: number;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
  onClose: () => void;
  aimMode: boolean;
  moveMode: boolean;
  onAimMode: (on: boolean) => void;
  timeMsRef: RefObject<number>;
  onSeek: (ms: number) => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_zoom" }>, "op" | "id">>) =>
    void onApply({ op: "update_zoom", id: zoom.id, ...patch });
  const mode = targetMode(zoom.target);
  const span = secOf(zoom.end_ms - zoom.start_ms);
  const seekIntoSpan = () => {
    const target = zoomScopedSeekMs(timeMsRef.current, zoom.start_ms, zoom.end_ms);
    if (target !== null) onSeek(target);
  };

  return (
    <InspectorShell kind="zoom">
      <InspectorHeader
        title="Zoom"
        range={spanRange(zoom.start_ms, zoom.end_ms)}
        deleteLabel="Delete zoom"
        onClose={onClose}
        onDelete={() => {
          void onApply({ op: "remove_zoom", id: zoom.id });
          onClose();
        }}
      />

      <Section title="Framing">
        <div className="e-ihero">
          <span className="e-ihero-v">
            {zoom.scale.toFixed(1)}
            <i>x</i>
          </span>
          <span className="e-ihero-l">Scale</span>
        </div>
        <Slider
          min={1}
          max={4}
          step={0.1}
          value={zoom.scale}
          onChange={(v) => upd({ scale: v })}
          accentColor="var(--e-zoom)"
          ariaLabel="Scale"
        />
        <div className="e-field e-iaim">
          <span className="e-fl">Target</span>
          <SegRow
            ariaLabel="Target"
            onPick={(k) => {
              if (k === "cursor") onAimMode(false);
              upd({ target: targetForMode(k as TargetMode, zoom.target) });
              seekIntoSpan();
            }}
            options={[
              { key: "cursor", label: "Follow cursor", on: mode === "cursor" },
              { key: "region", label: "Region", on: mode === "region" },
            ]}
          />
          {mode === "region" && (
            <button
              type="button"
              className={`e-aimbtn${aimMode ? " on" : ""}`}
              disabled={moveMode}
              title={
                moveMode
                  ? "Turn off Move in preview first"
                  : "Click or drag the preview to place this zoom's aim point"
              }
              onClick={() => onAimMode(!aimMode)}
            >
              <IconCrosshair size={14} />
              {aimMode ? "Aiming, click the preview" : "Aim on stage"}
            </button>
          )}
        </div>
        <Hint>Applies while this zoom is active, scrub inside it to preview.</Hint>
      </Section>

      <Section title="Timing" value={`${span.toFixed(2)}s of ${secOf(dur)}s`}>
        <ValueRow
          ariaLabel="Timing"
          cells={[
            {
              label: "Start",
              sec: secOf(zoom.start_ms),
              min: 0,
              max: secOf(zoom.end_ms),
              onChange: (v) => upd({ start_ms: Math.round(v * 1000) }),
            },
            {
              label: "End",
              sec: secOf(zoom.end_ms),
              min: secOf(zoom.start_ms),
              max: secOf(dur),
              onChange: (v) => upd({ end_ms: Math.round(v * 1000) }),
            },
            {
              label: "In",
              sec: secOf(zoom.zoom_in_ms),
              min: 0,
              max: span,
              onChange: (v) => upd({ zoom_in_ms: Math.round(v * 1000) }),
            },
            {
              label: "Out",
              sec: secOf(zoom.zoom_out_ms),
              min: 0,
              max: span,
              onChange: (v) => upd({ zoom_out_ms: Math.round(v * 1000) }),
            },
          ]}
        />
        <div className="e-field e-iaim">
          <span className="e-fl">Duration</span>
          <SegRow
            ariaLabel="Duration"
            options={durationOptions(!!zoom.smart_typing)}
            onPick={(k) => upd({ smart_typing: k === "smart" })}
          />
        </div>
        <Hint>
          {zoom.smart_typing
            ? "The end lands one hold after the last key of the typing that starts here, and refits whenever you move the start."
            : "Drag the block on the timeline to move it, or its right edge to change where it ends."}
        </Hint>
      </Section>

      <Section title="Motion" value={motionReadout(zoom.easing, zoom.easing_out)}>
        <MotionField
          input={zoomGraphInput(zoom, zooms)}
          easing={zoom.easing}
          easingOut={zoom.easing_out}
          onPatch={(p) =>
            upd({ easing: p.easing, easing_out: p.easing_out, zoom_in_ms: p.inMs, zoom_out_ms: p.outMs })
          }
        />
      </Section>

      <Section title="Webcam during zoom">
        <SegRow
          ariaLabel="Webcam during zoom"
          options={CAM_ACTION_OPTIONS.map((o) => ({
            key: o.label,
            label: o.label,
            on: isCamActionSelected(zoom.cam_action, o.value),
          }))}
          onPick={(k) => {
            const opt = CAM_ACTION_OPTIONS.find((o) => o.label === k);
            if (opt) {
              void onApply({ op: "set_zoom_cam_action", id: zoom.id, action: opt.value });
              seekIntoSpan();
            }
          }}
        />
      </Section>
    </InspectorShell>
  );
}
