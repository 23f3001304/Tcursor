import type { ClickFxSettings, SpotlightMode, VideoFxMode } from "../../hud/settings/settings";
import { Switch, Slider, Picker, Swatches } from "../controls/Controls";
import { SPOTLIGHT_TINTS, rgb, swatchItems } from "./effectSwatches";

const MODES: { value: SpotlightMode; label: string }[] = [
  { value: "classic", label: "Classic" },
  { value: "blur", label: "Blur" },
  { value: "halo", label: "Halo" },
  { value: "breathing", label: "Breathing" },
  { value: "nebula", label: "Nebula" },
  { value: "vignette", label: "Vignette" },
];

const VMODES: { value: VideoFxMode; label: string }[] = [
  { value: "nebulawash", label: "Nebula wash" },
  { value: "cinematicdim", label: "Cinematic" },
  { value: "screenfocus", label: "Screen focus" },
  { value: "colorpop", label: "Color pop" },
];

export function SpotlightSection({
  settings,
  set,
}: {
  settings: ClickFxSettings;
  set: <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => void;
}) {
  return (
    <>
      <div className="e-grp">
        <span className="e-sechead">Spotlight</span>
        <div className="e-switchrow">
          <span>Spotlight always on</span>
          <Switch
            on={settings.spotlight}
            onChange={(v) => set("spotlight", v)}
            ariaLabel="Spotlight always on"
          />
        </div>
        {settings.spotlight && (
          <>
            <div className="e-two">
              <div className="e-field">
                <span className="e-fl">Spotlight Mode</span>
                <Picker
                  value={settings.spotlight_mode}
                  options={MODES}
                  onChange={(v) => set("spotlight_mode", v)}
                  ariaLabel="Spotlight Mode"
                />
              </div>
              <div className="e-field">
                <span className="e-fl">Tint</span>
                <Swatches
                  items={swatchItems(SPOTLIGHT_TINTS)}
                  isSelected={(c) => rgb(c) === rgb(settings.spotlight_tint)}
                  onSelect={(c) => set("spotlight_tint", c)}
                />
              </div>
            </div>
            <div className="e-two">
              <div className="e-field">
                <Slider
                  min={0.2}
                  max={0.9}
                  step={0.05}
                  value={settings.spotlight_dim}
                  onChange={(v) => set("spotlight_dim", v)}
                  ariaLabel="Dim Override"
                  label="Dim Override"
                  formatValue={(v) => `${Math.round(v * 100)}%`}
                />
              </div>
              <div className="e-field">
                <Slider
                  min={0.05}
                  max={0.3}
                  step={0.01}
                  value={settings.spotlight_radius}
                  onChange={(v) => set("spotlight_radius", v)}
                  ariaLabel="Radius Override"
                  label="Radius Override"
                  formatValue={(v) => `${Math.round(v * 100)}%`}
                />
              </div>
            </div>
            <div className="e-field">
              <Slider
                min={0.02}
                max={0.25}
                step={0.01}
                value={settings.spotlight_feather}
                onChange={(v) => set("spotlight_feather", v)}
                ariaLabel="Feather"
                label="Feather"
                formatValue={(v) => `${Math.round(v * 100)}%`}
              />
            </div>
            <div className="e-switchrow">
              <span>Dim webcam</span>
              <Switch
                on={settings.spotlight_dim_camera}
                onChange={(v) => set("spotlight_dim_camera", v)}
                ariaLabel="Dim webcam"
              />
            </div>
          </>
        )}
      </div>

      <div className="e-grp">
        <span className="e-sechead">Video effect</span>
        <div className="e-field">
          <span className="e-fl">Video FX Mode</span>
          <Picker
            value={settings.video_fx_mode}
            options={VMODES}
            onChange={(v) => set("video_fx_mode", v)}
            ariaLabel="Video FX Mode"
          />
        </div>
      </div>
    </>
  );
}
