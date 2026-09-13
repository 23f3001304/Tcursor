import { useState } from "react";
import type { ZoomSettings } from "./settings";
import { Field, Switch, Range, Advanced } from "./SettingsControls";

const PRESETS: { name: string; v: Pick<ZoomSettings, "target_scale" | "smoothness" | "hold_ms"> }[] = [
  { name: "Subtle", v: { target_scale: 1.6, smoothness: 0.16, hold_ms: 1800 } },
  { name: "Balanced", v: { target_scale: 2.2, smoothness: 0.10, hold_ms: 2200 } },
  { name: "Punchy", v: { target_scale: 2.8, smoothness: 0.07, hold_ms: 1400 } },
];
const CLICKS = [1, 2, 3];

function activePreset(v: ZoomSettings): string {
  const p = PRESETS.find((p) => p.v.target_scale === v.target_scale && p.v.smoothness === v.smoothness && p.v.hold_ms === v.hold_ms);
  return p ? p.name : "Custom";
}

export function SettingsZoom({ value, onChange }: { value: ZoomSettings; onChange: (v: ZoomSettings) => void }) {
  const set = <K extends keyof ZoomSettings>(k: K, v: ZoomSettings[K]) => onChange({ ...value, [k]: v });
  const cur = activePreset(value);
  const [adv, setAdv] = useState(false);
  return (
    <section className="sec">
      <h3 className="sec-title">Auto-zoom</h3>
      <div className="sf"><div className="sf-row"><span className="sf-label">Zoom on click</span>
        <Switch on={value.enabled} onChange={(v) => set("enabled", v)} /></div></div>
      <Field label="Clicks to zoom">
        <div className="seg">
          {CLICKS.map((n) => (
            <button key={n} className={`seg-btn ${value.clicks === n ? "on" : ""}`} onClick={() => set("clicks", n)}>{n}</button>
          ))}
        </div>
      </Field>
      <Field label="Feel">
        <div className="seg" style={{ flexWrap: "wrap" }}>
          {PRESETS.map((p) => (
            <button key={p.name} className={`seg-btn ${cur === p.name ? "on" : ""}`} onClick={() => onChange({ ...value, ...p.v })}>{p.name}</button>
          ))}
          <button className={`seg-btn ${cur === "Custom" ? "on" : ""}`} onClick={() => setAdv(true)}>Custom</button>
        </div>
      </Field>
      <div style={{ fontSize: 11, fontWeight: 700, letterSpacing: 0.5, textTransform: "uppercase", opacity: 0.5, margin: "16px 2px 6px" }}>Smart zoom</div>
      <div className="sf"><div className="sf-row"><span className="sf-label">Smart type</span>
        <Switch on={value.smart_hold} onChange={(v) => set("smart_hold", v)} /></div></div>
      <div className="sf"><div className="sf-row"><span className="sf-label">Smart follow</span>
        <Switch on={value.smart_follow} onChange={(v) => set("smart_follow", v)} /></div></div>
      <Advanced open={adv} onToggle={setAdv}>
        <Range label="Amount" value={value.target_scale} min={1.2} max={4} step={0.1} onChange={(v) => set("target_scale", v)} fmt={(v) => `${v.toFixed(1)}x`} />
        <Range label="Smoothness" value={value.smoothness} min={0.04} max={0.3} step={0.01} onChange={(v) => set("smoothness", v)} fmt={(v) => v.toFixed(2)} />
        <Range label="Camera smoothing" value={value.camera_smoothing_ms} min={0} max={400} step={10} onChange={(v) => set("camera_smoothing_ms", Math.round(v))} fmt={(v) => (v === 0 ? "Off" : `${v} ms`)} />
        <span className="sf-hint" style={{ display: "block", marginTop: -6 }}>
          Smooths the auto-zoom camera's path. Higher is calmer but lags the cursor - 120 ms is a good start.
        </span>
        <Range label={value.smart_hold ? "Idle release" : "Hold"} value={value.hold_ms} min={600} max={5000} step={100} onChange={(v) => set("hold_ms", Math.round(v))} fmt={(v) => `${(v / 1000).toFixed(1)} s`} />
        <div className="sf"><div className="sf-row"><span className="sf-label">Shrink camera on zoom</span>
          <Switch on={value.camera_shrink} onChange={(v) => set("camera_shrink", v)} /></div></div>
        {value.camera_shrink && (
          <Range label="Min camera size" value={value.camera_shrink_min} min={0.3} max={1.0} step={0.02} onChange={(v) => set("camera_shrink_min", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        )}
      </Advanced>
    </section>
  );
}
