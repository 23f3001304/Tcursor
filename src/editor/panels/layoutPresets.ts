import type { LayoutSeg } from "../../lib/edit";
import type { AppearanceSettings, LayoutPreset } from "../../hud/settings/settings";
import { DEFAULT_APPEARANCE, type ModeKey } from "../../hud/preferences/appearanceFields";

// The Layouts panel's arithmetic, with no React in it: which layout the playhead is sitting in,
// whether a typed name may be saved, and the four list edits (save / rename / delete / apply).
// Everything here takes and returns a `Settings`-shaped value, so the panel's own job is reduced
// to calling `setSettings(next)` - the read-modify-write that keeps every OTHER settings field.

/** The built-in row: the five layouts exactly as they ship. Not stored, not deletable, always
 *  first, so a user who has wandered can always get back without having saved anything first. */
export const BUILTIN_PRESET: LayoutPreset = { id: "default", name: "Default", appearance: DEFAULT_APPEARANCE };

/** Longer than this and the row's name would have to ellipsise at 320px, which is exactly the
 *  truncated-name bug the panel pass fixed elsewhere - so it is refused at the source instead. */
export const MAX_PRESET_NAME = 40;

/** `LayoutSeg.layout` is a free `string` on the wire; only these five name an appearance block. */
const LAYOUT_KEYS: Record<string, ModeKey> = {
  screen: "screen", camera: "camera", presenter: "presenter",
  screen_only: "screen_only", camera_only: "camera_only",
};

/** Which layout the playhead is inside, for the panel's opening pick. The LAST segment covering
 *  `timeMs` wins (later segments are drawn over earlier ones), and anything else - no segments,
 *  a gap between them, a layout name this build does not know - reads as `screen`, the base
 *  layout every recording falls back to. Never writes anything: this only chooses what to show. */
export function layoutAtPlayhead(segs: LayoutSeg[], timeMs: number): ModeKey {
  let hit: ModeKey = "screen";
  for (const s of segs) {
    if (timeMs < s.start_ms || timeMs >= s.end_ms) continue;
    const key = LAYOUT_KEYS[s.layout];
    if (key) hit = key;
  }
  return hit;
}

/** Why this name cannot be saved, or `null` when it can. Compared case-insensitively and against
 *  the built-in row too, so "default" is taken. `exceptId` is the row being renamed (a rename to
 *  its own current name is not a clash). */
export function presetNameError(name: string, list: LayoutPreset[], exceptId?: string): string | null {
  const trimmed = name.trim();
  if (!trimmed) return "Give this look a name.";
  if (trimmed.length > MAX_PRESET_NAME) return `Keep the name under ${MAX_PRESET_NAME} characters.`;
  const taken = [BUILTIN_PRESET, ...list]
    .some((p) => p.id !== exceptId && p.name.trim().toLowerCase() === trimmed.toLowerCase());
  return taken ? `There is already a look called "${trimmed}".` : null;
}

/** The next free `lp<n>` id. Derived from the list rather than from a counter or a clock, so the
 *  same list always produces the same next id (which is what makes `addPreset` testable). */
export function nextPresetId(list: LayoutPreset[]): string {
  let n = 1;
  const used = new Set(list.map((p) => p.id));
  while (used.has(`lp${n}`)) n += 1;
  return `lp${n}`;
}

type WithPresets = { layout_presets: LayoutPreset[] };

/** Append `appearance` as a new saved look. Returns the settings unchanged when the name is not
 *  savable, so a caller that forgot to check `presetNameError` cannot write a duplicate. */
export function addPreset<S extends WithPresets>(settings: S, name: string, appearance: AppearanceSettings): S {
  const list = settings.layout_presets;
  if (presetNameError(name, list)) return settings;
  const preset: LayoutPreset = { id: nextPresetId(list), name: name.trim(), appearance };
  return { ...settings, layout_presets: [...list, preset] };
}

/** Rename one saved look in place (same id, same position, same appearance). Unchanged when the
 *  new name is not savable or the id names nothing. */
export function renamePreset<S extends WithPresets>(settings: S, id: string, name: string): S {
  const list = settings.layout_presets;
  if (presetNameError(name, list, id) || !list.some((p) => p.id === id)) return settings;
  return { ...settings, layout_presets: list.map((p) => (p.id === id ? { ...p, name: name.trim() } : p)) };
}

/** Drop one saved look. The built-in row is not in the list, so it can never be deleted. */
export function removePreset<S extends WithPresets>(settings: S, id: string): S {
  return { ...settings, layout_presets: settings.layout_presets.filter((p) => p.id !== id) };
}

/** One layout back to how it ships, leaving the other four exactly as the user left them. */
export function resetLayout(appearance: AppearanceSettings, mode: ModeKey): AppearanceSettings {
  return { ...appearance, [mode]: DEFAULT_APPEARANCE[mode] };
}
