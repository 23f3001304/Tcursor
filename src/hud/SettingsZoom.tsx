import type { ZoomSettings } from "./settings";
import { Field, Switch, Slider } from "./SettingsControls";

const CLICKS = [1, 2, 3];

export function SettingsZoom({ value, onChange }: { value: ZoomSettings; onChange: (v: ZoomSettings) => void }) {
  const set = <K extends keyof ZoomSettings>(k: K, v: ZoomSettings[K]) => onChange({ ...value, [k]: v });
  return (
    <section className="sec">
      <h3 className="sec-title">Auto-zoom</h3>
      <div className="sf"><div className="sf-row"><span className="sf-label">Zoom on click</span>
        <Switch on={value.enabled} onChange={(v) => set("enabled", v)} /></div></div>
      <Field label="Clicks to zoom" hint={value.clicks > 1 ? `${value.clicks} quick clicks` : "any click"}>
        <div className="seg">
          {CLICKS.map((n) => (
            <button key={n} className={`seg-btn ${value.clicks === n ? "on" : ""}`} onClick={() => set("clicks", n)}>{n}</button>
          ))}
        </div>
      </Field>
      <Field label="Amount" hint={`${value.target_scale.toFixed(1)}×`}>
        <Slider value={value.target_scale} min={1.2} max={4} step={0.1} onChange={(v) => set("target_scale", v)} fmt={(v) => `${v.toFixed(1)}×`} />
      </Field>
      <Field label="Hold" hint={`${(value.hold_ms / 1000).toFixed(1)} s`}>
        <Slider value={value.hold_ms} min={600} max={5000} step={100} onChange={(v) => set("hold_ms", Math.round(v))} fmt={(v) => `${(v / 1000).toFixed(1)} s`} />
      </Field>
      <Field label="Smoothness" hint={value.smoothness.toFixed(2)}>
        <Slider value={value.smoothness} min={0.04} max={0.3} step={0.01} onChange={(v) => set("smoothness", v)} fmt={(v) => v.toFixed(2)} />
      </Field>
    </section>
  );
}
