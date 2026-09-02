import type { CSSProperties } from "react";
import type { ClickFxSettings, ClickFxStyle, SpotlightMode, VideoFxMode } from "./settings";
import { Field, Switch, Range, Advanced } from "./SettingsControls";

const STYLES: [ClickFxStyle, string][] = [["none", "None"], ["ripple", "Ripple"], ["pulse", "Pulse"], ["glow", "Glow"], ["shockwave", "Shockwave"], ["particles", "Particles"], ["neon", "Neon"]];
const MODES: [SpotlightMode, string][] = [["classic", "Classic"], ["blur", "Blur"], ["halo", "Halo"], ["breathing", "Breathing"], ["nebula", "Nebula"], ["vignette", "Vignette"]];
const VMODES: [VideoFxMode, string][] = [["nebulawash", "Nebula wash"], ["cinematicdim", "Cinematic"], ["screenfocus", "Screen focus"], ["colorpop", "Color pop"]];
const SWATCHES: [number, number, number][] = [[255, 255, 255], [239, 68, 68], [59, 130, 246], [34, 197, 94], [250, 204, 21]];
const TINTS: [number, number, number][] = [[130, 90, 255], [59, 130, 246], [34, 197, 94], [239, 68, 68], [250, 204, 21]];
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const groupStyle: CSSProperties = { fontSize: 11, fontWeight: 700, letterSpacing: 0.5, textTransform: "uppercase", opacity: 0.5, margin: "16px 2px 6px" };

function Picker<T extends string>({ items, on, pick }: { items: [T, string][]; on: T; pick: (v: T) => void }) {
  return (
    <div className="optg">
      {items.map(([v, label]) => (
        <button key={v} type="button" className={`opt ${on === v ? "on" : ""}`} onClick={() => pick(v)}>{label}</button>
      ))}
    </div>
  );
}

function Swatches({ items, on, pick }: { items: [number, number, number][]; on: [number, number, number]; pick: (c: [number, number, number]) => void }) {
  return (
    <div className="swatches">
      {items.map((c) => (
        <button key={rgb(c)} className={`swatch ${rgb(on) === rgb(c) ? "on" : ""}`} style={{ background: rgb(c) }} onClick={() => pick(c)} aria-label={rgb(c)} />
      ))}
    </div>
  );
}

export function SettingsClickFx({ value, onChange }: { value: ClickFxSettings; onChange: (v: ClickFxSettings) => void }) {
  const set = <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => onChange({ ...value, [k]: v });
  return (
    <section className="sec">
      {/* No top-level "Effects" h3 here (unlike the other tabs) - it sat directly above "Click
          effects" with nothing between them, reading as a duplicated, empty header. The FX tab
          is already the fully-labeled group below; the "FX" tab pill is title enough. */}
      <div style={groupStyle}>Click effects</div>
      <div className="sf"><div className="sf-row"><span className="sf-label">Show click effects</span>
        <Switch on={value.enabled} onChange={(v) => set("enabled", v)} /></div></div>
      {value.enabled && (<>
        <Field label="Style"><Picker items={STYLES} on={value.style} pick={(s) => set("style", s)} /></Field>
        <Field label="Color"><Swatches items={SWATCHES} on={value.color} pick={(c) => set("color", c)} /></Field>
        <Range label="Intensity" value={value.intensity} min={0.2} max={1} step={0.05} onChange={(v) => set("intensity", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
      </>)}
      <div className="sf"><div className="sf-row"><span className="sf-label">Keystroke captions</span>
        <Switch on={value.captions} onChange={(v) => set("captions", v)} /></div></div>

      <div style={groupStyle}>Cursor spotlight</div>
      <div className="sf"><div className="sf-row"><span className="sf-label">Always on</span>
        <Switch on={value.spotlight} onChange={(v) => set("spotlight", v)} /></div></div>
      <Field label="Mode" hint="also used by the hotkey"><Picker items={MODES} on={value.spotlight_mode} pick={(m) => set("spotlight_mode", m)} /></Field>
      <Field label="Tint"><Swatches items={TINTS} on={value.spotlight_tint} pick={(c) => set("spotlight_tint", c)} /></Field>
      <Advanced>
        <Range label="Dim" value={value.spotlight_dim} min={0.2} max={0.9} step={0.05} onChange={(v) => set("spotlight_dim", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        <Range label="Radius" value={value.spotlight_radius} min={0.05} max={0.30} step={0.01} onChange={(v) => set("spotlight_radius", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
        <Range label="Feather" value={value.spotlight_feather} min={0.02} max={0.25} step={0.01} onChange={(v) => set("spotlight_feather", v)} fmt={(v) => `${Math.round(v * 100)}%`} />
      </Advanced>

      <div style={groupStyle}>Video effect</div>
      <Field label="Mode" hint="hold hotkey to activate"><Picker items={VMODES} on={value.video_fx_mode} pick={(m) => set("video_fx_mode", m)} /></Field>
    </section>
  );
}
