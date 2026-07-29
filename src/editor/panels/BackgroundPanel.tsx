import { useState } from "react";
import { PanelHeader } from "./PanelHeader";
import type { EditDoc } from "../../lib/edit";
import type { BackgroundSettings } from "../../hud/settings/settings";
import { Slider } from "../controls/Controls";
import { COLOR_PRESETS, GRADIENT_PRESETS, ACCENTS } from "./backgroundPresets";

type BgTab = "default" | "color" | "gradient";
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const DEFAULT_BG: BackgroundSettings = {
  kind: "mesh", solid: [24, 24, 30],
  gradient_from: [36, 41, 56], gradient_to: [88, 64, 120], gradient_angle_deg: 135, blur: 0,
};

export function BackgroundPanel({
  doc,
  onSaveSettings,
  onClose,
}: {
  doc: EditDoc;
  onSaveSettings: (nextSettings: EditDoc["settings"]) => void;
  onClose: () => void;
}) {
  const app = doc.settings.appearance;
  const bg = doc.settings.background;
  const padPct = Math.round((app.screen?.pad ?? 0.03125) * 100);
  const radiusPx = Math.round((app.screen?.screen_radius ?? 0.016) * 1000);
  // Which preset grid is showing. Derived from the saved kind so reopening the panel lands on
  // the right tab, but merely BROWSING the image/video tabs (no real backend) never saves -
  // only picking a color/gradient swatch below does.
  const [tab, setTab] = useState<BgTab>(bg.kind === "solid" ? "color" : bg.kind === "gradient" ? "gradient" : "default");

  const setBg = (patch: Partial<BackgroundSettings>) =>
    onSaveSettings({ ...doc.settings, background: { ...bg, ...patch } });

  const handleReset = () => {
    setTab("default");
    onSaveSettings({
      ...doc.settings,
      background: DEFAULT_BG,
      appearance: { ...app, screen: { ...app.screen, pad: 0.03125, screen_radius: 0.016 } },
      ui: { ...doc.settings.ui, accent: [239, 68, 68] },
    });
  };

  const setPad = (v: number) => {
    const frac = v / 100;
    onSaveSettings({ ...doc.settings, appearance: { ...app, screen: { ...app.screen, pad: frac } } });
  };

  const setRadius = (v: number) => {
    const frac = v / 1000;
    onSaveSettings({ ...doc.settings, appearance: { ...app, screen: { ...app.screen, screen_radius: frac } } });
  };

  const setAccent = (c: [number, number, number]) => {
    onSaveSettings({ ...doc.settings, ui: { ...doc.settings.ui, accent: c } });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Background" lede="Frame padding, corner radius, and background style."
        onReset={handleReset} onClose={onClose} />

      {/* Background Type Selector */}
      <div className="e-field">
        <span className="e-sechead">Background Type</span>
        <div className="e-seg">
          {(["default", "color", "gradient"] as BgTab[]).map((t) => (
            <button key={t} type="button" className={tab === t ? "on" : ""}
              onClick={() => setTab(t)} style={{ textTransform: "capitalize" }}>
              {t}
            </button>
          ))}
        </div>
      </div>

      {tab === "default" && (
        <div className="e-field">
          <span className="e-sechead">Presets</span>
          <div className="e-preset-grid">
            <button type="button" className={`e-preset-circle ${bg.kind === "mesh" ? "on" : ""}`}
              title="Default" style={{ background: "linear-gradient(135deg, #242938, #58406f)" }}
              onClick={() => setBg({ kind: "mesh" })} />
          </div>
        </div>
      )}

      {tab === "color" && (
        <div className="e-field">
          <span className="e-sechead">Presets</span>
          <div className="e-preset-grid">
            {COLOR_PRESETS.map((c, idx) => {
              const isSelected = bg.kind === "solid" && rgb(bg.solid) === rgb(c);
              return (
                <button key={idx} type="button" className={`e-preset-circle ${isSelected ? "on" : ""}`}
                  style={{ background: rgb(c) }} onClick={() => setBg({ kind: "solid", solid: c })} />
              );
            })}
          </div>
        </div>
      )}

      {tab === "gradient" && (
        <div className="e-field">
          <span className="e-sechead">Presets</span>
          <div className="e-preset-grid">
            {GRADIENT_PRESETS.map((g, idx) => {
              const isSelected = bg.kind === "gradient" && rgb(bg.gradient_from) === rgb(g.from) && rgb(bg.gradient_to) === rgb(g.to);
              return (
                <button key={idx} type="button" className={`e-preset-circle ${isSelected ? "on" : ""}`}
                  style={{ background: `linear-gradient(${g.angle}deg, ${rgb(g.from)}, ${rgb(g.to)})` }}
                  onClick={() => setBg({ kind: "gradient", gradient_from: g.from, gradient_to: g.to, gradient_angle_deg: g.angle })} />
              );
            })}
          </div>
        </div>
      )}

      {/* Accent Colors */}
      <div className="e-field">
        <span className="e-sechead">Accent Colors</span>
        <div className="e-accent-list">
          {ACCENTS.map((c) => {
            const colorStr = rgb(c);
            const isSelected = rgb(doc.settings.ui.accent) === colorStr;
            return (
              <button
                key={colorStr}
                type="button"
                className={`e-accent-circle ${isSelected ? "on" : ""}`}
                style={{ background: colorStr }}
                onClick={() => setAccent(c)}
              />
            );
          })}
        </div>
      </div>

      {/* Custom Sliders */}
      <div className="e-field">
        <span className="e-fl">Background Blur <b>{Math.round(bg.blur * 100)}%</b></span>
        <Slider min={0} max={100} step={5} value={Math.round(bg.blur * 100)} onChange={(v) => setBg({ blur: v / 100 })} />
      </div>

      <div className="e-field">
        <span className="e-fl">Corner Radius <b>{radiusPx}px</b></span>
        <Slider min={0} max={80} step={1} value={radiusPx} onChange={setRadius} />
      </div>

      <div className="e-field">
        <span className="e-fl">Padding <b>{padPct}%</b></span>
        <Slider min={0} max={25} step={1} value={padPct} onChange={setPad} />
      </div>
    </div>
  );
}
