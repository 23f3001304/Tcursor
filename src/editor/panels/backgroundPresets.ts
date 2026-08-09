// Preset swatches for BackgroundPanel. Each entry is plain RGB (or a 2-stop RGB gradient),
// the exact shape `settings.background` stores - so a swatch always renders identically to
// what gets applied (no CSS-only preset that the backend can't reproduce). `name` (Task 26 -
// promoted from what used to be a same-line comment) becomes each swatch's aria-label.
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

export interface GradientPreset { from: [number, number, number]; to: [number, number, number]; angle: number }
export const GRADIENT_PRESETS: GradientPreset[] = [
  { from: [15, 32, 39], to: [44, 83, 100], angle: 135 },
  { from: [131, 58, 180], to: [253, 29, 29], angle: 135 },
  { from: [254, 140, 0], to: [248, 54, 0], angle: 135 },
  { from: [33, 147, 176], to: [109, 213, 237], angle: 135 },
  { from: [238, 156, 167], to: [255, 221, 225], angle: 135 },
  { from: [0, 198, 255], to: [0, 114, 255], angle: 135 },
  { from: [17, 153, 142], to: [56, 239, 125], angle: 135 },
  { from: [255, 153, 102], to: [255, 94, 98], angle: 135 },
  { from: [255, 226, 89], to: [255, 167, 81], angle: 135 },
  { from: [122, 28, 172], to: [225, 0, 255], angle: 135 },
  { from: [31, 64, 55], to: [153, 242, 200], angle: 135 },
];

export const ACCENTS: NamedColor[] = [
  { rgb: [239, 68, 68], name: "Red" },
  { rgb: [124, 108, 240], name: "Purple" },
  { rgb: [59, 130, 246], name: "Blue" },
  { rgb: [34, 197, 94], name: "Green" },
  { rgb: [245, 158, 11], name: "Orange" },
];
