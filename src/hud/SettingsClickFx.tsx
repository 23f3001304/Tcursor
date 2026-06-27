import type { ClickFxSettings, ClickFxStyle } from "./settings";
import { Field, Switch, Slider } from "./SettingsControls";

const STYLES: ClickFxStyle[] = ["none", "ripple", "pulse"];
const SWATCHES: [number, number, number][] = [[255, 255, 255], [239, 68, 68], [59, 130, 246], [34, 197, 94], [250, 204, 21]];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

export function SettingsClickFx({ value, onChange }: { value: ClickFxSettings; onChange: (v: ClickFxSettings) => void }) {
  const set = <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => onChange({ ...value, [k]: v });
  return (
    <section className="sec">
      <h3 className="sec-title">Click FX</h3>
      <div className="sf"><div className="sf-row"><span className="sf-label">Show click effects</span>
        <Switch on={value.enabled} onChange={(v) => set("enabled", v)} /></div></div>
      <Field label="Style">
        <div className="seg">
          {STYLES.map((s) => (
            <button key={s} className={`seg-btn ${value.style === s ? "on" : ""}`} onClick={() => set("style", s)}>{s}</button>
          ))}
        </div>
      </Field>
      <Field label="Color">
        <div className="swatches">
          {SWATCHES.map((c) => (
            <button key={rgb(c)} className={`swatch ${rgb(value.color) === rgb(c) ? "on" : ""}`} style={{ background: rgb(c) }} onClick={() => set("color", c)} aria-label={rgb(c)} />
          ))}
        </div>
      </Field>
      <Field label="Intensity" hint={`${Math.round(value.intensity * 100)}%`}>
        <Slider value={value.intensity} min={0.2} max={1} step={0.05} onChange={(v) => set("intensity", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
      </Field>
      <div className="sf"><div className="sf-row"><span className="sf-label">Keystroke captions</span>
        <Switch on={value.captions} onChange={(v) => set("captions", v)} /></div></div>
      <div className="sf"><div className="sf-row"><span className="sf-label">Cursor spotlight</span>
        <Switch on={value.spotlight} onChange={(v) => set("spotlight", v)} /></div></div>
    </section>
  );
}
