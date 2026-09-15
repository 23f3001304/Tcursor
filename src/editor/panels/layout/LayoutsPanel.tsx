import { useEffect, useRef, useState, type RefObject } from "react";
import { PanelHeader } from "../PanelHeader";
import { LayoutKnobs } from "./LayoutKnobs";
import { LayoutMiniPreview } from "./LayoutMiniPreview";
import { LayoutPresetList } from "./LayoutPresetList";
import { Picker } from "../../controls/Controls";
import { getSettings, setSettings } from "../../../shared/ipc";
import type { EditDoc } from "../../../shared/edit";
import type {
  AppearanceSettings,
  LayoutPreset,
  ModeAppearance,
  Settings,
} from "../../../hud/settings/settings";
import { MODES, DEFAULT_APPEARANCE, type ModeKey } from "../../../hud/preferences/appearanceFields";
import { addPreset, layoutAtPlayhead, removePreset, renamePreset, resetLayout } from "./layoutPresets";

const MODE_OPTS = MODES.map(([value, label]) => ({ value, label }));

export function LayoutsPanel({
  doc,
  timeMsRef,
  onSaveSettings,
  onClose,
}: {
  doc: EditDoc;
  timeMsRef: RefObject<number>;
  onSaveSettings: (s: EditDoc["settings"]) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<ModeKey>(() => layoutAtPlayhead(doc.layout, timeMsRef.current));
  const [app, setApp] = useState<Settings | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    getSettings()
      .then((s) => {
        if (alive.current) setApp(s);
      })
      .catch(() => {});
    return () => {
      alive.current = false;
    };
  }, []);

  const appearance: AppearanceSettings = doc.settings.appearance ?? DEFAULT_APPEARANCE;
  const ma: ModeAppearance = appearance[mode] ?? DEFAULT_APPEARANCE[mode];
  const saveAppearance = (next: AppearanceSettings) => onSaveSettings({ ...doc.settings, appearance: next });
  const writeApp = (next: Settings) => {
    setApp(next);
    setSettings(next).catch(() => {});
  };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Layouts" lede="How each layout looks, and looks you can save." onClose={onClose} />

      <div className="e-grp">
        <Picker value={mode} options={MODE_OPTS} onChange={setMode} ariaLabel="Layout" />
        <LayoutMiniPreview mode={mode} ma={ma} />
      </div>

      <LayoutKnobs mode={mode} ma={ma} onChange={(next) => saveAppearance({ ...appearance, [mode]: next })} />

      <button
        type="button"
        className="e-lay-txt e-lay-reset"
        onClick={() => saveAppearance(resetLayout(appearance, mode))}
      >
        Reset this layout
      </button>

      <LayoutPresetList
        presets={app?.layout_presets ?? []}
        appearance={appearance}
        onApply={(p: LayoutPreset) => saveAppearance(p.appearance)}
        onSave={(name) => {
          if (app) writeApp(addPreset(app, name, appearance));
        }}
        onRename={(id, name) => {
          if (app) writeApp(renamePreset(app, id, name));
        }}
        onDelete={(id) => {
          if (app) writeApp(removePreset(app, id));
        }}
        onMakeDefault={(a) => {
          if (app) writeApp({ ...app, appearance: a });
        }}
      />
    </div>
  );
}
