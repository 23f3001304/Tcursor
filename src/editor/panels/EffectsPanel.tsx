import { PanelHeader } from "./PanelHeader";
import type { ClickFxSettings, ClickFxStyle, SpotlightMode, VideoFxMode } from "../../hud/settings/settings";
import { Switch, Slider, Picker, Swatches, type SwatchItem } from "../controls/Controls";
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

// Mirrors the HUD's SettingsClickFx (src/hud/settings/SettingsClickFx.tsx) - same 4 modes, same
// labels/order - so the hotkey-driven video effect looks the same whichever panel set it.
const VMODES: { value: VideoFxMode; label: string }[] = [
  { value: "nebulawash", label: "Nebula wash" },
  { value: "cinematicdim", label: "Cinematic" },
  { value: "screenfocus", label: "Screen focus" },
  { value: "colorpop", label: "Color pop" },
];

// [color, human name] - the name becomes each swatch's aria-label.
const SWATCHES: [[number, number, number], string][] = [
  [[255, 255, 255], "White"], [[239, 68, 68], "Red"], [[59, 130, 246], "Blue"], [[34, 197, 94], "Green"], [[245, 158, 11], "Orange"],
];
// Mirrors the HUD's SettingsClickFx TINTS exactly (values only - the HUD's own swatches aren't named either).
const TINTS: [[number, number, number], string][] = [
  [[130, 90, 255], "Violet"], [[59, 130, 246], "Blue"], [[34, 197, 94], "Green"], [[239, 68, 68], "Red"], [[250, 204, 21], "Yellow"],
];

const rgb = (c: [number, number, number]) => `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
const swatchItems = (colors: [[number, number, number], string][]): SwatchItem<[number, number, number]>[] =>
  colors.map(([c, name]) => ({ key: rgb(c), css: rgb(c), value: c, ariaLabel: name }));

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
  // Style/color/intensity are Style-dependent: with no ripple style there's nothing for a color
  // or intensity to apply to, so they're disabled (not hidden) rather than vanishing the moment
  // "None" is picked - the picker that got you there stays put either way.
  const styleless = settings.style === "none";

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
            <Picker value={settings.style} options={STYLES} onChange={(v) => set("style", v)} ariaLabel="Ripple Style" />
          </div>

          <div className="e-field">
            <span className="e-fl">Ripple Color</span>
            <Swatches items={swatchItems(SWATCHES)} isSelected={(c) => rgb(c) === rgb(settings.color)}
              onSelect={(c) => set("color", c)} disabled={styleless} />
          </div>

          <div className="e-field">
            <Slider min={0.2} max={1.0} step={0.05} value={settings.intensity} disabled={styleless}
              onChange={(v) => set("intensity", v)} ariaLabel="Intensity" label="Intensity" formatValue={(v) => `${Math.round(v * 100)}%`} />
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
            <Picker value={settings.spotlight_mode} options={MODES} onChange={(v) => set("spotlight_mode", v)} ariaLabel="Spotlight Mode" />
          </div>

          <div className="e-field">
            <span className="e-fl">Tint</span>
            <Swatches items={swatchItems(TINTS)} isSelected={(c) => rgb(c) === rgb(settings.spotlight_tint)}
              onSelect={(c) => set("spotlight_tint", c)} />
          </div>

          <div className="e-field">
            <Slider min={0.2} max={0.9} step={0.05} value={settings.spotlight_dim}
              onChange={(v) => set("spotlight_dim", v)} ariaLabel="Dim Override" label="Dim Override" formatValue={(v) => `${Math.round(v * 100)}%`} />
          </div>

          <div className="e-field">
            <Slider min={0.05} max={0.3} step={0.01} value={settings.spotlight_radius}
              onChange={(v) => set("spotlight_radius", v)} ariaLabel="Radius Override" label="Radius Override" formatValue={(v) => `${Math.round(v * 100)}%`} />
          </div>

          <div className="e-field">
            <Slider min={0.02} max={0.25} step={0.01} value={settings.spotlight_feather}
              onChange={(v) => set("spotlight_feather", v)} ariaLabel="Feather" label="Feather" formatValue={(v) => `${Math.round(v * 100)}%`} />
          </div>

          <div className="e-switchrow">
            <span>Dim webcam</span>
            <Switch on={settings.spotlight_dim_camera} onChange={(v) => set("spotlight_dim_camera", v)} />
          </div>
        </>
      )}

      {/* Video effect - a separate, hotkey-activated overlay; independent of the spotlight toggle. */}
      <div className="e-sec">
        <div className="e-field" style={{ marginBottom: 0 }}>
          <span className="e-fl">Video FX Mode</span>
          <Picker value={settings.video_fx_mode} options={VMODES} onChange={(v) => set("video_fx_mode", v)} ariaLabel="Video FX Mode" />
        </div>
      </div>
    </div>
  );
}
