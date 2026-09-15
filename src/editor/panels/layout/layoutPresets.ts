import type { LayoutSeg } from "../../../shared/edit";
import type { AppearanceSettings, LayoutPreset } from "../../../hud/settings/settings";
import { DEFAULT_APPEARANCE, type ModeKey } from "../../../hud/preferences/appearanceFields";

export const BUILTIN_PRESET: LayoutPreset = {
  id: "default",
  name: "Default",
  appearance: DEFAULT_APPEARANCE,
};

export const MAX_PRESET_NAME = 40;

const LAYOUT_KEYS: Record<string, ModeKey> = {
  screen: "screen",
  camera: "camera",
  presenter: "presenter",
  screen_only: "screen_only",
  camera_only: "camera_only",
};

export function layoutAtPlayhead(segs: LayoutSeg[], timeMs: number): ModeKey {
  let hit: ModeKey = "screen";
  for (const s of segs) {
    if (timeMs < s.start_ms || timeMs >= s.end_ms) continue;
    const key = LAYOUT_KEYS[s.layout];
    if (key) hit = key;
  }
  return hit;
}

export function presetNameError(name: string, list: LayoutPreset[], exceptId?: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) return "Give this look a name.";
  if (trimmed.length > MAX_PRESET_NAME) return `Keep the name under ${MAX_PRESET_NAME} characters.`;
  const taken = [BUILTIN_PRESET, ...list].some(
    (p) => p.id !== exceptId && p.name.trim().toLowerCase() === trimmed.toLowerCase(),
  );
  return taken ? `There is already a look called "${trimmed}".` : null;
}

export function nextPresetId(list: LayoutPreset[]): string {
  let n = 1;
  const used = new Set(list.map((p) => p.id));
  while (used.has(`lp${n}`)) n += 1;
  return `lp${n}`;
}

type WithPresets = { layout_presets: LayoutPreset[] };

export function addPreset<S extends WithPresets>(
  settings: S,
  name: string,
  appearance: AppearanceSettings,
): S {
  const list = settings.layout_presets;
  if (presetNameError(name, list)) return settings;
  const preset: LayoutPreset = { id: nextPresetId(list), name: name.trim(), appearance };
  return { ...settings, layout_presets: [...list, preset] };
}

export function renamePreset<S extends WithPresets>(settings: S, id: string, name: string): S {
  const list = settings.layout_presets;
  if (presetNameError(name, list, id) || !list.some((p) => p.id === id)) return settings;
  return { ...settings, layout_presets: list.map((p) => (p.id === id ? { ...p, name: name.trim() } : p)) };
}

export function removePreset<S extends WithPresets>(settings: S, id: string): S {
  return { ...settings, layout_presets: settings.layout_presets.filter((p) => p.id !== id) };
}

export function resetLayout(appearance: AppearanceSettings, mode: ModeKey): AppearanceSettings {
  return { ...appearance, [mode]: DEFAULT_APPEARANCE[mode] };
}
