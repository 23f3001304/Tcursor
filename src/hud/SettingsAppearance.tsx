import { useState } from "react";
import type { AppearanceSettings, ModeAppearance } from "./settings";
import { Field, Range } from "./SettingsControls";
import { MODES, MODE_SLIDERS, MODE_HAS_SHAPE, MODE_HAS_CORNER, SLIDERS, SHAPES, CORNERS, pct, type ModeKey } from "./appearanceFields";

/** Even pill grid (modes) - reuses the FX tab's `.optg`/`.opt` styles. */
function Pills<T extends string>({ items, on, pick }: { items: [T, string][]; on: T; pick: (v: T) => void }) {
  return (<div className="optg">{items.map(([v, l]) => (
    <button key={v} type="button" className={`opt ${on === v ? "on" : ""}`} onClick={() => pick(v)}>{l}</button>
  ))}</div>);
}

/** Segmented control (shape / corner). */
function Seg<T extends string>({ items, on, pick }: { items: [T, string][]; on: T; pick: (v: T) => void }) {
  return (<div className="seg">{items.map(([v, l]) => (
    <button key={v} type="button" className={`seg-btn ${on === v ? "on" : ""}`} onClick={() => pick(v)}>{l}</button>
  ))}</div>);
}

export function SettingsAppearance({ value, onChange }: { value: AppearanceSettings; onChange: (v: AppearanceSettings) => void }) {
  const [mode, setMode] = useState<ModeKey>("screen");
  const ma = value[mode];
  const set = <K extends keyof ModeAppearance>(k: K, v: ModeAppearance[K]) => onChange({ ...value, [mode]: { ...ma, [k]: v } });
  return (
    <section className="sec">
      <h3 className="sec-title">Frame appearance</h3>
      <Pills items={MODES} on={mode} pick={setMode} />
      {MODE_SLIDERS[mode].map((k) => {
        const s = SLIDERS[k];
        return <Range key={k} label={s.label} value={ma[k]} min={s.min} max={s.max} step={s.step}
          onChange={(v) => set(k, v)} fmt={pct} />;
      })}
      {MODE_HAS_SHAPE[mode] && (<>
        <Field label="Webcam shape"><Seg items={SHAPES} on={ma.cam_shape} pick={(v) => set("cam_shape", v)} /></Field>
        {ma.cam_shape === "rounded" && (
          <Range label="Webcam corner" value={ma.cam_radius} min={SLIDERS.cam_radius.min} max={SLIDERS.cam_radius.max}
            step={SLIDERS.cam_radius.step} onChange={(v) => set("cam_radius", v)} fmt={pct} />
        )}
      </>)}
      {MODE_HAS_CORNER[mode] && (
        <Field label="Webcam corner"><Seg items={CORNERS} on={ma.cam_corner} pick={(v) => set("cam_corner", v)} /></Field>
      )}
    </section>
  );
}
