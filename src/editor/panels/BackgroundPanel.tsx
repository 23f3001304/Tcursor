import { useState } from "react";
import { PanelHeader } from "./PanelHeader";
import type { EditDoc } from "../../lib/edit";
import type { BackgroundSettings } from "../../hud/settings/settings";
import { Slider, Swatches, type SwatchItem } from "../controls/Controls";
import { COLOR_PRESETS, GRADIENT_PRESETS, ACCENTS, type GradientPreset } from "./backgroundPresets";

type BgTab = "default" | "color" | "gradient";
const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const DEFAULT_BG: BackgroundSettings = {
  kind: "mesh", solid: [24, 24, 30],
  gradient_from: [36, 41, 56], gradient_to: [88, 64, 120], gradient_angle_deg: 135, blur: 0,
};

// `ariaLabel` uses each preset's own name (COLOR_PRESETS/ACCENTS carry one; the gradients don't,
// so those swatches fall back to Swatches' default - their CSS gradient string).
const COLOR_ITEMS: SwatchItem<[number, number, number]>[] = COLOR_PRESETS.map((p) => ({ key: rgb(p.rgb), css: rgb(p.rgb), value: p.rgb, ariaLabel: p.name }));
const GRADIENT_ITEMS: SwatchItem<GradientPreset>[] = GRADIENT_PRESETS.map((g, i) => ({
  key: `g${i}`, css: `linear-gradient(${g.angle}deg, ${rgb(g.from)}, ${rgb(g.to)})`, value: g,
}));
const ACCENT_ITEMS: SwatchItem<[number, number, number]>[] = ACCENTS.map((p) => ({ key: rgb(p.rgb), css: rgb(p.rgb), value: p.rgb, ariaLabel: p.name }));

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
  // the right tab, but merely BROWSING to the "default" tab never saves - only picking a
  // color/gradient swatch (or the "default" tab's own mesh swatch) below does.
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
        // No "Presets" header here (unlike the color/gradient tabs below) - `mesh` is a single
        // bundled image (`BackgroundKind::Mesh` in Rust, see `settings.ts`'s doc comment), not a
        // data-driven set like COLOR_PRESETS/GRADIENT_PRESETS that backgroundPresets.ts could
        // expand, so a section label over the one swatch just restated the tab name for no
        // reason. The swatch itself still renders at the same size/position every other preset
        // tab uses, via the shared `.e-preset-grid`/`.e-preset-circle` classes.
        <div className="e-field">
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
          <Swatches items={COLOR_ITEMS} variant="preset"
            isSelected={(c) => bg.kind === "solid" && rgb(bg.solid) === rgb(c)}
            onSelect={(c) => setBg({ kind: "solid", solid: c })} />
        </div>
      )}

      {tab === "gradient" && (
        <div className="e-field">
          <span className="e-sechead">Presets</span>
          <Swatches items={GRADIENT_ITEMS} variant="preset"
            isSelected={(g) => bg.kind === "gradient" && rgb(bg.gradient_from) === rgb(g.from) && rgb(bg.gradient_to) === rgb(g.to)}
            onSelect={(g) => setBg({ kind: "gradient", gradient_from: g.from, gradient_to: g.to, gradient_angle_deg: g.angle })} />
        </div>
      )}

      {/* Accent Colors */}
      <div className="e-field">
        <span className="e-sechead">Accent Colors</span>
        <Swatches items={ACCENT_ITEMS} variant="accent"
          isSelected={(c) => rgb(doc.settings.ui.accent) === rgb(c)} onSelect={setAccent} />
      </div>

      {/* Custom Sliders */}
      <div className="e-field">
        <Slider min={0} max={100} step={5} value={Math.round(bg.blur * 100)} onChange={(v) => setBg({ blur: v / 100 })} ariaLabel="Background Blur"
          label="Background Blur" formatValue={(v) => `${Math.round(v)}%`} />
      </div>

      <div className="e-field">
        <Slider min={0} max={80} step={1} value={radiusPx} onChange={setRadius} ariaLabel="Corner Radius"
          label="Corner Radius" formatValue={(v) => `${Math.round(v)}px`} />
      </div>

      <div className="e-field">
        <Slider min={0} max={25} step={1} value={padPct} onChange={setPad} ariaLabel="Padding"
          label="Padding" formatValue={(v) => `${Math.round(v)}%`} />
      </div>
    </div>
  );
}
