import { PanelHeader } from "./PanelHeader";
import type { ClickFxSettings, ClickFxStyle, SpotlightMode } from "../../hud/settings/settings";
import { Switch, Slider, Picker } from "../controls/Controls";
import { EffectPills } from "./EffectPills";

const STYLES: { value: ClickFxStyle; label: string }[] = [
  { value: "none", label: "None" },
  { value: "ripple", label: "Ripple" },
  { value: "pulse", label: "Pulse" },
  { value: "glow", label: "Glow" },
  { value: "shockwave", label: "Shockwave" },
  { value: "particles", label: "Particles" },
  { value: "neon", label: "Neon" },
];

const MODES: { value: SpotlightMode; label: string }[] = [
  { value: "classic", label: "Classic" },
  { value: "blur", label: "Blur" },
  { value: "halo", label: "Halo" },
  { value: "breathing", label: "Breathing" },
  { value: "nebula", label: "Nebula" },
  { value: "vignette", label: "Vignette" },
];

const SWATCHES: [number, number, number][] = [
  [255, 255, 255], [239, 68, 68], [59, 130, 246], [34, 197, 94], [245, 158, 11]
];

const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;

export function EffectsPanel({
  settings,
  onChange,
  onClose,
  onAddZoom,
  onAddSpotlight,
  onAddLayout,
  onAddCameraMove,
}: {
  settings: ClickFxSettings;
  onChange: (v: ClickFxSettings) => void;
  onClose: () => void;
  onAddZoom: () => void;
  onAddSpotlight: () => void;
  onAddLayout: () => void;
  onAddCameraMove: () => void;
}) {
  const set = <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Effects" lede="Drag pills to the timeline, or tune click ripples and spotlight." onClose={onClose} />

      <EffectPills onAddZoom={onAddZoom} onAddSpotlight={onAddSpotlight} onAddLayout={onAddLayout} onAddCameraMove={onAddCameraMove} />

      {/* Click ripples */}
      <div className="e-sec">
        <div className="e-switchrow">
          <span>Click animations</span>
          <Switch on={settings.enabled} onChange={(v) => set("enabled", v)} />
        </div>
      </div>

      {settings.enabled && (
        <>
          <div className="e-field">
            <span className="e-fl">Ripple Style</span>
            <Picker value={settings.style} options={STYLES} onChange={(v) => set("style", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Ripple Color</span>
            <div style={{ display: "flex", gap: 8, marginTop: 2 }}>
              {SWATCHES.map((c) => {
                const colorStr = rgb(c);
                const isSelected = rgb(settings.color) === colorStr;
                return (
                  <button
                    key={colorStr}
                    type="button"
                    style={{
                      background: colorStr,
                      border: isSelected ? "2px solid var(--e-fg)" : "1px solid var(--e-border)",
                      width: 22,
                      height: 22,
                      borderRadius: "50%",
                      cursor: "pointer",
                      padding: 0,
                      outline: "none"
                    }}
                    onClick={() => set("color", c)}
                  />
                );
              })}
            </div>
          </div>

          <div className="e-field">
            <span className="e-fl">Intensity <b>{Math.round(settings.intensity * 100)}%</b></span>
            <Slider
              min={0.2}
              max={1.0}
              step={0.05}
              value={settings.intensity}
              onChange={(v) => set("intensity", v)}
            />
          </div>
        </>
      )}

      {/* Spotlight options */}
      <div className="e-sec">
        <div className="e-switchrow">
          <span>Spotlight always on</span>
          <Switch on={settings.spotlight} onChange={(v) => set("spotlight", v)} />
        </div>
      </div>

      {settings.spotlight && (
        <>
          <div className="e-field">
            <span className="e-fl">Spotlight Mode</span>
            <Picker value={settings.spotlight_mode} options={MODES} onChange={(v) => set("spotlight_mode", v)} />
          </div>

          <div className="e-field">
            <span className="e-fl">Dim Override <b>{Math.round(settings.spotlight_dim * 100)}%</b></span>
            <Slider
              min={0.2}
              max={0.9}
              step={0.05}
              value={settings.spotlight_dim}
              onChange={(v) => set("spotlight_dim", v)}
            />
          </div>

          <div className="e-field">
            <span className="e-fl">Radius Override <b>{Math.round(settings.spotlight_radius * 100)}%</b></span>
            <Slider
              min={0.05}
              max={0.3}
              step={0.01}
              value={settings.spotlight_radius}
              onChange={(v) => set("spotlight_radius", v)}
            />
          </div>
        </>
      )}
    </div>
  );
}
