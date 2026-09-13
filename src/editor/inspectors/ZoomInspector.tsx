import type { RefObject } from "react";
import { IconCrosshair } from "@tabler/icons-react";
import { PanelHeader } from "../panels/PanelHeader";
import type { CamZoomAction, EditDoc, EditOp, Zoom, ZoomTarget } from "../../lib/edit";
import { NumberField, Slider } from "../controls/Controls";
import { CurveEditor } from "./CurveEditor";
import { Hint, InspectorShell, RemoveButton, SegRow, Section, TimingRow, secOf, spanLede } from "./InspectorShape";

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

export const PRESETS = [
  { name: "Subtle", scale: 1.6, zoom_in_ms: 400, zoom_out_ms: 500, easing: "smooth" },
  { name: "Balanced", scale: 2.2, zoom_in_ms: 350, zoom_out_ms: 450, easing: "smooth" },
  { name: "Punchy", scale: 2.8, zoom_in_ms: 200, zoom_out_ms: 300, easing: "spring" },
];

/** The preset a zoom currently matches on all four fields, or "Custom". */
export function activePreset(z: Pick<Zoom, "scale" | "zoom_in_ms" | "zoom_out_ms" | "easing">): string {
  const p = PRESETS.find((p) => Math.abs(p.scale - z.scale) < 0.05 && p.zoom_in_ms === z.zoom_in_ms
    && p.zoom_out_ms === z.zoom_out_ms && p.easing === z.easing);
  return p ? p.name : "Custom";
}

export function ZoomInspector({ zoom, dur, onApply, onClose, aimMode, moveMode, onAimMode, timeMsRef, onSeek }: {
  zoom: Zoom; dur: number; onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
  aimMode: boolean; moveMode: boolean; onAimMode: (on: boolean) => void;
  timeMsRef: RefObject<number>; onSeek: (ms: number) => void;
}) {
  const upd = (patch: Partial<Omit<Extract<EditOp, { op: "update_zoom" }>, "op" | "id">>) =>
    void onApply({ op: "update_zoom", id: zoom.id, ...patch });
  const mode = targetMode(zoom.target);
  const cur = activePreset(zoom);
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
      <PanelHeader title="Zoom" lede={spanLede(zoom.start_ms, zoom.end_ms)} closeTitle="Deselect" onClose={onClose} />

      <Section title="Timing">
        <TimingRow startMs={zoom.start_ms} endMs={zoom.end_ms} durMs={dur}
          onStart={(start_ms) => upd({ start_ms })} onEnd={(end_ms) => upd({ end_ms })} />
        <Hint>Drag the block on the timeline to move it.</Hint>
      </Section>

      <Section title="Framing">
        <label className="e-field">
          <Slider min={1} max={4} step={0.1} value={zoom.scale} onChange={(v) => upd({ scale: v })}
            accentColor="var(--e-zoom)" ariaLabel="Scale" label="Scale" formatValue={(v) => `${v.toFixed(1)}x`} />
        </label>
        <div className="e-field">
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

      <Section title="Webcam during zoom">
        <SegRow ariaLabel="Webcam during zoom"
          options={CAM_ACTION_OPTIONS.map((o) => ({ key: o.label, label: o.label, on: isCamActionSelected(zoom.cam_action, o.value) }))}
          onPick={(k) => {
            const opt = CAM_ACTION_OPTIONS.find((o) => o.label === k);
            if (opt) { void onApply({ op: "set_zoom_cam_action", id: zoom.id, action: opt.value }); seekIntoSpan(); }
          }} />
      </Section>

      <Section title="Feel" value={cur}>
        <SegRow ariaLabel="Feel" options={PRESETS.map((p) => ({ key: p.name, label: p.name, on: cur === p.name }))}
          onPick={(k) => { const p = PRESETS.find((p) => p.name === k); if (p) upd({ scale: p.scale, zoom_in_ms: p.zoom_in_ms, zoom_out_ms: p.zoom_out_ms, easing: p.easing }); }} />
        <div className="e-field2" style={{ marginTop: 12 }}>
          <label className="e-field"><span className="e-fl">Zoom in</span>
            <NumberField step={0.05} min={0} max={span} value={secOf(zoom.zoom_in_ms)}
              onChange={(v) => upd({ zoom_in_ms: Math.round(v * 1000) })} /></label>
          <label className="e-field"><span className="e-fl">Zoom out</span>
            <NumberField step={0.05} min={0} max={span} value={secOf(zoom.zoom_out_ms)}
              onChange={(v) => upd({ zoom_out_ms: Math.round(v * 1000) })} /></label>
        </div>
        <CurveEditor value={zoom.easing} onChange={(easing) => upd({ easing })} />
      </Section>

      <RemoveButton label="Delete zoom" onClick={() => { void onApply({ op: "remove_zoom", id: zoom.id }); onClose(); }} />
    </InspectorShell>
  );
}
