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

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Captions" lede="Show which hotkey was held as an on-screen caption." onClose={onClose} />

      <div className="e-sec">
        <div className="e-switchrow">
          <span>Show keystrokes on screen</span>
          <Switch on={settings.captions} onChange={(v) => set("captions", v)} />
        </div>
      </div>

      <p className="e-lede" style={{ marginTop: 12 }}>
        Auto-generated subtitles need speech-to-text and are coming soon.
      </p>
    </div>
  );
}
