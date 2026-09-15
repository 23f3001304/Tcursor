import { PanelHeader } from "./PanelHeader";
import type { ClickFxSettings, ClickFxStyle } from "../../hud/settings/settings";
import { Switch, Slider, Picker, Swatches, Disclosure } from "../controls/Controls";
import { EffectPills } from "./EffectPills";
import { SpotlightSection } from "./SpotlightSection";
import { RIPPLE_COLORS, rgb, swatchItems } from "./effectSwatches";

const STYLES: { value: ClickFxStyle; label: string }[] = [
  { value: "none", label: "None" },
  { value: "ripple", label: "Ripple" },
  { value: "pulse", label: "Pulse" },
  { value: "glow", label: "Glow" },
  { value: "shockwave", label: "Shockwave" },
  { value: "particles", label: "Particles" },
  { value: "neon", label: "Neon" },
];

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
  const styleless = settings.style === "none";

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="Effects"
        lede="Add to the timeline, or tune the always-on effects."
        onClose={onClose}
      />

      <EffectPills
        onAddZoom={onAddZoom}
        onAddSpotlight={onAddSpotlight}
        onAddLayout={onAddLayout}
        onAddCameraMove={onAddCameraMove}
      />

      <div className="e-grp">
        <span className="e-sechead">Clicks</span>
        <div className="e-switchrow">
          <span>Click animations</span>
          <Switch on={settings.enabled} onChange={(v) => set("enabled", v)} ariaLabel="Click animations" />
        </div>
        {settings.enabled && (
          <>
            <div className="e-field">
              <span className="e-fl">Ripple Style</span>
              <Picker
                value={settings.style}
                options={STYLES}
                onChange={(v) => set("style", v)}
                ariaLabel="Ripple Style"
              />
            </div>
            <div className="e-field">
              <span className="e-fl">Ripple Color</span>
              <Swatches
                items={swatchItems(RIPPLE_COLORS)}
                isSelected={(c) => rgb(c) === rgb(settings.color)}
                onSelect={(c) => set("color", c)}
                disabled={styleless}
              />
            </div>
            <div className="e-field">
              <Slider
                min={0.2}
                max={1.0}
                step={0.05}
                value={settings.intensity}
                disabled={styleless}
                onChange={(v) => set("intensity", v)}
                ariaLabel="Intensity"
                label="Intensity"
                formatValue={(v) => `${Math.round(v * 100)}%`}
              />
            </div>
          </>
        )}
      </div>

      <Disclosure id="effects">
        <SpotlightSection settings={settings} set={set} />
      </Disclosure>
    </div>
  );
}
