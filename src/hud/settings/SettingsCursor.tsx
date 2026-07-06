import type { CursorSettings, CursorStyle } from "./settings";
import { Field, Switch, Range } from "./SettingsControls";

const STYLES: [CursorStyle, string][] = [["system", "System"], ["enhanced", "Enhanced"], ["hidden", "Hidden"]];

export function SettingsCursor({ value, onChange }: { value: CursorSettings; onChange: (v: CursorSettings) => void }) {
  const set = <K extends keyof CursorSettings>(k: K, v: CursorSettings[K]) => onChange({ ...value, [k]: v });
  return (
    <section className="sec">
      <h3 className="sec-title">Cursor</h3>
      <Field label="Style" hint="Enhanced redraws a smooth pointer">
        <div className="seg">{STYLES.map(([v, l]) => (
          <button key={v} type="button" className={`seg-btn ${value.style === v ? "on" : ""}`} onClick={() => set("style", v)}>{l}</button>
        ))}</div>
      </Field>
      {value.style === "enhanced" && (<>
        <Range label="Size" value={value.size} min={0.5} max={2.5} step={0.05} onChange={(v) => set("size", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        <Range label="Motion blur" value={value.motion_blur} min={0} max={1} step={0.05} onChange={(v) => set("motion_blur", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        <div className="sf"><div className="sf-row"><span className="sf-label">Click bounce</span>
          <Switch on={value.click_bounce} onChange={(v) => set("click_bounce", v)} /></div></div>
        {value.click_bounce && (
          <Range label="Bounce intensity" value={value.bounce_intensity} min={0} max={1} step={0.05} onChange={(v) => set("bounce_intensity", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        )}
      </>)}
    </section>
  );
}
