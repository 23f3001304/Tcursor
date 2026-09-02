import { PanelHeader } from "./PanelHeader";
import type { ClickFxSettings } from "../../hud/settings/settings";
import { Switch } from "../controls/Controls";

export function CaptionsPanel({
  settings,
  onChange,
  onClose,
}: {
  settings: ClickFxSettings;
  onChange: (v: ClickFxSettings) => void;
  onClose: () => void;
}) {
  const set = <K extends keyof ClickFxSettings>(k: K, v: ClickFxSettings[K]) => {
    onChange({ ...settings, [k]: v });
  };
  const handleReset = () => set("captions", false); // Rust ClickFxSettings::default(): captions: false

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Captions" lede="Show which hotkey was held as an on-screen caption."
        onReset={handleReset} onClose={onClose} />

      {/* `.e-field`, not `.e-sec` - `.e-sec` adds its own top border + margin/padding, which
          stacked with PanelHeader's own hairline to read as an empty divider strip with nothing
          in it. Every other panel's first group after PanelHeader uses `.e-field` directly. */}
      <div className="e-field">
        <div className="e-switchrow">
          <span>Show keystrokes on screen</span>
          <Switch on={settings.captions} onChange={(v) => set("captions", v)} />
        </div>
        <span className="e-lede" style={{ marginTop: 8, marginBottom: 0 }}>
          Spoken captions (auto-transcription) are coming later — this toggle shows typed keystrokes.
        </span>
      </div>
    </div>
  );
}
