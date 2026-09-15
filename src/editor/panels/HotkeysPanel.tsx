import { PanelHeader } from "./PanelHeader";
import type { ClickFxSettings } from "../../hud/settings/settings";
import { Switch } from "../controls/Controls";

export function HotkeysPanel({
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
  const handleReset = () => set("captions", false);

  return (
    <div className="e-panel e-insp">
      <PanelHeader
        title="Hotkeys"
        lede="Show the hotkeys you press, on screen."
        onReset={handleReset}
        onClose={onClose}
      />

      <div className="e-grp">
        <div className="e-switchrow">
          <span>Show keystrokes on screen</span>
          <Switch
            on={settings.captions}
            onChange={(v) => set("captions", v)}
            ariaLabel="Show keystrokes on screen"
          />
        </div>
        <span className="e-hintline">Captions for what you say live on the Captions tab.</span>
      </div>
    </div>
  );
}
