// Preset swatches for BackgroundPanel's Color tab and the accent row. Each entry is plain RGB,
// the exact shape `settings.background.solid` / `settings.ui.accent` store - so a swatch always
// renders identically to what gets applied (no CSS-only preset that the backend can't reproduce).
// `name` (Task 26 - promoted from what used to be a same-line comment) becomes each swatch's
// aria-label. The GRADIENT presets moved to Rust (`settings::wallpapers::GRADIENT_WALLPAPERS`)
// when they gained thumbnails and an optional middle stop: the panel now reads them off
// `background_thumbs` instead of keeping a second copy here that could drift from the render.
import type { BackgroundSettings } from "../../hud/settings/settings";

/** `BackgroundSettings::default()` on the Rust side, field for field - what a fresh project has
 *  and what the panel's Reset lands on. `asset: null` clears the CHOICE, not the file: Reset never
 *  deletes an imported background (that is the card's own Remove action). */
export const DEFAULT_BG: BackgroundSettings = {
  kind: "mesh", solid: [24, 24, 30], mesh: "", gradient_mid: null,
  gradient_from: [36, 41, 56], gradient_to: [88, 64, 120], gradient_angle_deg: 135,
  blur: 0, asset: null, dim: 0,
};

export interface NamedColor { rgb: [number, number, number]; name: string }
export const COLOR_PRESETS: NamedColor[] = [
  { rgb: [9, 9, 11], name: "Dark Zinc" },
  { rgb: [24, 24, 27], name: "Zinc 900" },
  { rgb: [45, 26, 74], name: "Deep Violet" },
  { rgb: [20, 61, 44], name: "Emerald Forest" },
  { rgb: [30, 41, 59], name: "Slate 800" },
  { rgb: [88, 28, 135], name: "Grape Purple" },
  { rgb: [124, 45, 18], name: "Warm Rust" },
  { rgb: [49, 46, 129], name: "Indigo Midnight" },
  { rgb: [15, 118, 110], name: "Teal 700" },
  { rgb: [190, 24, 93], name: "Magenta Rose" },
  { rgb: [55, 65, 81], name: "Steel Gray" },
  { rgb: [17, 24, 39], name: "Charcoal" },
];

export const ACCENTS: NamedColor[] = [
  { rgb: [239, 68, 68], name: "Red" },
  { rgb: [124, 108, 240], name: "Purple" },
  { rgb: [59, 130, 246], name: "Blue" },
  { rgb: [34, 197, 94], name: "Green" },
  { rgb: [245, 158, 11], name: "Orange" },
];
