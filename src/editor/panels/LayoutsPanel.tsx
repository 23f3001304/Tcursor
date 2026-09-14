import { useEffect, useRef, useState, type RefObject } from "react";
import { PanelHeader } from "./PanelHeader";
import { LayoutKnobs } from "./LayoutKnobs";
import { LayoutMiniPreview } from "./LayoutMiniPreview";
import { LayoutPresetList } from "./LayoutPresetList";
import { Segmented } from "../controls/Controls";
import { getSettings, setSettings } from "../../lib/ipc";
import type { EditDoc } from "../../lib/edit";
import type { AppearanceSettings, LayoutPreset, ModeAppearance, Settings } from "../../hud/settings/settings";
import { MODES, DEFAULT_APPEARANCE, type ModeKey } from "../../hud/preferences/appearanceFields";
import { addPreset, layoutAtPlayhead, removePreset, renamePreset, resetLayout } from "./layoutPresets";

const MODE_OPTS = MODES.map(([value, label]) => ({ value, label }));

/** How each of the five layouts looks, and looks you can save.
 *
 *  Two things live here that used to have no home. The per-layout knobs: the Camera panel edited
 *  the `screen` layout's webcam and nothing else, so the other four layouts' appearance was
 *  reachable only from the HUD's own settings - this panel edits all five, one at a time, against a
 *  schematic of the layout being edited. And saved looks: a snapshot of ALL FIVE layouts under a
 *  name, stored in the app config (`Settings.layout_presets`) rather than in `edit.json`, which is
 *  what makes a look reusable on the next recording.
 *
 *  Two stores, deliberately: the knobs write the PROJECT (`doc.settings.appearance`, through
 *  `saveDocSettings` - one undo step per change, live on the stage via the `rev` bump), and the
 *  preset list writes the APP (`get_settings`/`set_settings`, read-modify-write so every other
 *  global field survives). Applying a look is the one place they meet: it reads the app and writes
 *  the project. */
export function LayoutsPanel({ doc, timeMsRef, onSaveSettings, onClose }: {
  doc: EditDoc;
  /** The live playhead as a ref (never a prop that ticks), read ONCE on mount to pick the layout
   *  the user is looking at. Re-picking on every tick would yank the panel out from under them. */
  timeMsRef: RefObject<number>;
  onSaveSettings: (s: EditDoc["settings"]) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<ModeKey>(() => layoutAtPlayhead(doc.layout, timeMsRef.current));
  // The app config, as last read or written. `null` until the first read lands, which is the only
  // state in which the preset actions do nothing: writing a half-read config would drop fields.
  const [app, setApp] = useState<Settings | null>(null);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    getSettings().then((s) => { if (alive.current) setApp(s); }).catch(() => {});
    return () => { alive.current = false; };
  }, []);

  const appearance: AppearanceSettings = doc.settings.appearance ?? DEFAULT_APPEARANCE;
  const ma: ModeAppearance = appearance[mode] ?? DEFAULT_APPEARANCE[mode];
  const saveAppearance = (next: AppearanceSettings) => onSaveSettings({ ...doc.settings, appearance: next });
  const writeApp = (next: Settings) => { setApp(next); setSettings(next).catch(() => {}); };

  return (
    <div className="e-panel e-insp">
      <PanelHeader title="Layouts" lede="How each layout looks, and looks you can save." onClose={onClose} />

      <div className="e-grp">
        <Segmented value={mode} options={MODE_OPTS} onChange={setMode} ariaLabel="Layout" columns={2} />
        <LayoutMiniPreview mode={mode} ma={ma} />
      </div>

      <LayoutKnobs mode={mode} ma={ma} onChange={(next) => saveAppearance({ ...appearance, [mode]: next })} />

      <button type="button" className="e-lay-txt e-lay-reset"
        onClick={() => saveAppearance(resetLayout(appearance, mode))}>
        Reset this layout
      </button>

      <LayoutPresetList presets={app?.layout_presets ?? []} appearance={appearance}
        onApply={(p: LayoutPreset) => saveAppearance(p.appearance)}
        onSave={(name) => { if (app) writeApp(addPreset(app, name, appearance)); }}
        onRename={(id, name) => { if (app) writeApp(renamePreset(app, id, name)); }}
        onDelete={(id) => { if (app) writeApp(removePreset(app, id)); }}
        onMakeDefault={(a) => { if (app) writeApp({ ...app, appearance: a }); }} />
    </div>
  );
}
