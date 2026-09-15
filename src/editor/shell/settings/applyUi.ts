import type { InterfaceSettings, Settings } from "../../../hud/settings/settings";
import { applyTheme } from "../../../hud/preferences/applyTheme";
import { getSettings, setSettings } from "../../../shared/ipc";

export function applyProjectUi(ui: InterfaceSettings): void {
  applyTheme(ui.theme, ui.accent);
}

export interface AppSettingsIo {
  getSettings: () => Promise<Settings>;
  setSettings: (s: Settings) => Promise<void>;
}

export function mirrorUiToApp(
  ui: InterfaceSettings,
  io: AppSettingsIo = { getSettings, setSettings },
): Promise<void> {
  return io
    .getSettings()
    .then((s) => io.setSettings({ ...s, ui: { ...s.ui, theme: ui.theme, accent: ui.accent } }))
    .catch(() => {});
}
