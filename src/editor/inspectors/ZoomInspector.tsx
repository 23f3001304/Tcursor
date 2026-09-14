import type { RefObject } from "react";
import { IconCrosshair } from "@tabler/icons-react";
import type { CamZoomAction, EditDoc, EditOp, Zoom, ZoomTarget } from "../../lib/edit";
import { Disclosure, Slider } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";
import {
  Hint, InspectorHeader, InspectorShell, SegRow, type SegOption, Section, ValueRow, secOf, spanRange,
} from "./InspectorShape";
import { FEEL_PRESETS, activeFeel, feelPatch } from "./zoomFeel";

/** The two target modes the picker offers. `ZoomTarget::Fixed{x,y}` is the whole Region model -
 *  the old "Center" button was only ever `fixed{0.5,0.5}`, so a doc written by it selects Region
 *  with its reticle already at the centre and needs no migration. */
export type TargetMode = "cursor" | "region";
export const targetMode = (t: ZoomTarget): TargetMode => (t === "cursor" ? "cursor" : "region");
/** Switching TO Region keeps whatever point is already stored, so toggling Follow cursor -> Region
 *  -> Follow cursor never silently discards an aim the user placed; a zoom that has only ever
 *  followed the cursor starts at frame centre. */
export function targetForMode(mode: TargetMode, current: ZoomTarget): ZoomTarget {
  if (mode === "cursor") return "cursor";
  return typeof current === "object" ? current : { fixed: { x: 0.5, y: 0.5 } };
}

// The per-zoom webcam override (`Zoom.cam_action`, `set_zoom_cam_action` op) - `null` inherits
// the global `settings.zoom.cam_zoom_default` (see `resolvedCamDefault`, stage/camZoomAction.ts).
// `Shrink.to` mirrors the Rust `ZoomSettings::camera_shrink_min` default (0.62).
export const CAM_ACTION_OPTIONS: { label: string; value: CamZoomAction | null }[] = [
  { label: "Global default", value: null },
  { label: "Stay", value: "stay" },
  { label: "Shrink", value: { shrink: { to: 0.62 } } },
  { label: "Hide", value: "hide" },
];

/** Whether `option` is the one currently in effect for `current` (`Zoom.cam_action`). The
 *  `Shrink` variant carries a `to` fraction that isn't user-editable here, so any Shrink value
 *  counts as a match - it's the only object-shaped variant. */
export function isCamActionSelected(current: CamZoomAction | null | undefined, option: CamZoomAction | null): boolean {
  if (option === null) return current == null;
  if (typeof option === "string") return current === option;
  return typeof current === "object" && current !== null && "shrink" in current;
}

/** The Target and "Webcam during zoom" controls only have a visible effect while the playhead is
 *  inside the zoom's own span - changing them elsewhere leaves the preview looking unchanged,
 *  which reads as "this setting doesn't work" (gate finding, live debug). `null` while `nowMs` is
 *  already inside `[startMs, endMs]`; otherwise the span's midpoint. */
export function zoomScopedSeekMs(nowMs: number, startMs: number, endMs: number): number | null {
  if (nowMs >= startMs && nowMs <= endMs) return null;
  return Math.round((startMs + endMs) / 2);
}

/** The Timing row's duration switch: a fixed end, or one that follows the typing after the start
 *  (the backend refits `end_ms` from `typing.json`; see `ops::smart_zoom`). */
export function durationOptions(smart: boolean): SegOption[] {
  return [
    { key: "fixed", label: "Fixed", on: !smart, title: "The end stays where you put it" },
    { key: "smart", label: "Smart typing", on: smart, title: "The end follows the typing after the start" },
  ];
}

export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode, timeMsRef, onSeek }: {
  zoom: Zoom; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
  aimMode: boolean; moveMode: boolean; onAimMode: (on: boolean) => void;
  timeMsRef: RefObject<number>; onSeek: (ms: number) => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_zoom" }>, "op" | "id">>) =>
    void onApply({ op: "update_zoom", id: zoom.id, ...patch });
  const mode = targetMode(zoom.target);
  const feel = activeFeel(zoom);
  const span = secOf(zoom.end_ms - zoom.start_ms);
  // Discoverability fix (live debug: "none of these settings work" was wiring working but
  // invisible outside the span) - jump the playhead into the zoom whenever a scoped control
  // changes while scrubbed outside it, so the effect is on screen immediately.
  const seekIntoSpan = () => {
    const target = zoomScopedSeekMs(timeMsRef.current, zoom.start_ms, zoom.end_ms);
    if (target !== null) onSeek(target);
  };

  return (
    <InspectorShell kind="zoom">
      <InspectorHeader title="Zoom" range={spanRange(zoom.start_ms, zoom.end_ms)}
        deleteLabel="Delete zoom" onClose={onClose}
        onDelete={() => { void onApply({ op: "remove_zoom", id: zoom.id }); onClose(); }} />

      <Section title="Framing">
        <div className="e-ihero">
          <span className="e-ihero-v">{zoom.scale.toFixed(1)}<i>x</i></span>
          <span className="e-ihero-l">Scale</span>
        </div>
        <Slider min={1} max={4} step={0.1} value={zoom.scale} onChange={(v) => upd({ scale: v })}
          accentColor="var(--e-zoom)" ariaLabel="Scale" />
        <div className="e-field e-iaim">
          <span className="e-fl">Target</span>
          <SegRow ariaLabel="Target" onPick={(k) => {
            if (k === "cursor") onAimMode(false);
            upd({ target: targetForMode(k as TargetMode, zoom.target) }); seekIntoSpan();
          }} options={[
            { key: "cursor", label: "Follow cursor", on: mode === "cursor" },
            { key: "region", label: "Region", on: mode === "region" },
          ]} />
          {mode === "region" && (
            <button type="button" className={`e-aimbtn${aimMode ? " on" : ""}`} disabled={moveMode}
              title={moveMode ? "Turn off Move in preview first" : "Click or drag the preview to place this zoom's aim point"}
              onClick={() => onAimMode(!aimMode)}>
              <IconCrosshair size={14} />{aimMode ? "Aiming, click the preview" : "Aim on stage"}
            </button>
          )}
        </div>
        <Hint>Applies while this zoom is active, scrub inside it to preview.</Hint>
      </Section>

      <Section title="Timing" value={`${span.toFixed(2)}s of ${secOf(dur)}s`}>
        <ValueRow ariaLabel="Timing" cells={[
          { label: "Start", sec: secOf(zoom.start_ms), min: 0, max: secOf(zoom.end_ms),
            onChange: (v) => upd({ start_ms: Math.round(v * 1000) }) },
          { label: "End", sec: secOf(zoom.end_ms), min: secOf(zoom.start_ms), max: secOf(dur),
            onChange: (v) => upd({ end_ms: Math.round(v * 1000) }) },
          { label: "In", sec: secOf(zoom.zoom_in_ms), min: 0, max: span,
            onChange: (v) => upd({ zoom_in_ms: Math.round(v * 1000) }) },
          { label: "Out", sec: secOf(zoom.zoom_out_ms), min: 0, max: span,
            onChange: (v) => upd({ zoom_out_ms: Math.round(v * 1000) }) },
        ]} />
        <div className="e-field e-iaim">
          <span className="e-fl">Duration</span>
          <SegRow ariaLabel="Duration" options={durationOptions(!!zoom.smart_typing)} onPick={(k) => upd({ smart_typing: k === "smart" })} />
        </div>
        <Hint>{zoom.smart_typing
          ? "The end lands one hold after the last key of the typing that starts here, and refits whenever you move the start."
          : "Drag the block on the timeline to move it, or its right edge to change where it ends."}</Hint>
      </Section>

      {/* The readout says "Custom" only when nothing is lit - with a preset matched the row already
          says which one, and repeating it on the heading row would be the same word twice. */}
      <Section title="Feel" value={feel ? undefined : "Custom"}>
        <SegRow ariaLabel="Feel" options={FEEL_PRESETS.map((p) => ({ key: p.name, label: p.name, on: feel === p.name }))}
          onPick={(k) => { const patch = feelPatch(k); if (patch) upd(patch); }} />
        <Disclosure id="zoom-feel" label="Custom">
          <CurveEditor value={zoom.easing} onChange={(easing) => upd({ easing })} />
        </Disclosure>
      </Section>

      <Section title="Webcam during zoom">
        <SegRow ariaLabel="Webcam during zoom"
          options={CAM_ACTION_OPTIONS.map((o) => ({ key: o.label, label: o.label, on: isCamActionSelected(zoom.cam_action, o.value) }))}
          onPick={(k) => {
            const opt = CAM_ACTION_OPTIONS.find((o) => o.label === k);
            if (opt) { void onApply({ op: "set_zoom_cam_action", id: zoom.id, action: opt.value }); seekIntoSpan(); }
          }} />
      </Section>
    </InspectorShell>
  );
}
