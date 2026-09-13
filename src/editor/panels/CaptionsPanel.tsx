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
      <PanelHeader title="Captions" lede="Show the hotkeys you press, on screen."
        onReset={handleReset} onClose={onClose} />

      {/* One switch and the sentence that qualifies it - no section heading, because a heading
          over a single row is chrome with nothing to organise. */}
      <div className="e-grp">
        <div className="e-switchrow">
          <span>Show keystrokes on screen</span>
          <Switch on={settings.captions} onChange={(v) => set("captions", v)} ariaLabel="Show keystrokes on screen" />
        </div>
        <span className="e-hintline">
          Spoken captions (auto-transcription) are coming later. This toggle shows typed keystrokes.
        </span>
      </div>
    </div>
  );
}
