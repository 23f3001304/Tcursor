# src/hud/preferences/appearanceFields.ts

Static configuration tables that drive the Appearance settings panel. Every slider range, mode-to-knob mapping, shape/corner visibility flag, and factory default lives here so UI components remain free of magic numbers. Changing the visual design language means editing this file, not hunting through JSX.

## ModeKey

```ts
export type ModeKey = "screen" | "camera" | "presenter" | "screen_only" | "camera_only";
```

Union of the five layout modes the HUD can be in. Used as `Record` keys throughout this file and as the discriminant in `AppearanceSettings`. A string union rather than an enum because the values are used verbatim as JSON keys sent to Tauri and as CSS class names.

### Used by

- `src/hud/settings/settings.ts` - `AppearanceSettings` is indexed by this type
- `src/hud/settings/SettingsAppearance.tsx` - selects which knobs to render for the active mode
- `src/hud/components/LayoutPreview.tsx` - picks the preview geometry
- `src/hud/Hud.tsx` - drives the active layout mode

## MODES

```ts
export const MODES: [ModeKey, string][] = [
  ["screen", "Screen"], ["camera", "Camera"], ["presenter", "Presenter"],
  ["screen_only", "S-only"], ["camera_only", "C-only"],
]
```

Ordered `[value, display-label]` pairs for the mode segmented control. The visual left-to-right order is exactly the array order, so reordering here changes the UI without touching any component.

## Knob

```ts
export type Knob = "pad" | "screen_size" | "screen_radius" | "cam_size" | "cam_radius" | "cam_margin_x" | "cam_margin_y";
```

Union of every adjustable slider key. Constrains `SLIDERS` and `MODE_SLIDERS` to a shared namespace so adding a new slider requires updating both the type and the table, preventing silent omissions.

## SLIDERS

```ts
export const SLIDERS: Record<Knob, { label: string; min: number; max: number; step: number }>
```

Maps each `Knob` to its display label and numeric range. All values are normalized fractions (0-1 of the output dimension), matching how `ModeAppearance` fields are stored. Slider components read `SLIDERS[knob]` at render time rather than inlining bounds.

Ranges by knob:
- `pad` 0-0.08 step 0.002 - padding around the composite frame
- `screen_size` 0.6-1.0 step 0.01 - fraction of output occupied by the screen panel
- `screen_radius` 0-0.05 step 0.002 - corner roundness of the screen panel
- `cam_size` 0.08-1.0 step 0.01 - fraction of output occupied by the camera overlay
- `cam_radius` 0-0.5 step 0.01 - corner roundness of the camera overlay (0 = sharp, 0.5 = full circle when shape is `rounded`)
- `cam_margin_x` / `cam_margin_y` 0-0.1 step 0.002 - gap between the camera bubble and the frame edge, horizontal and vertical

### Used by

- `src/hud/settings/SettingsAppearance.tsx` - renders one range input per active knob

## MODE_SLIDERS

```ts
export const MODE_SLIDERS: Record<ModeKey, Knob[]>
```

Per-mode ordered list of which knobs are active. Derived from the design matrix: `camera_only` omits all screen knobs; `screen_only` omits all camera knobs; `presenter` shows only `pad` and `screen_radius`. The panel iterates this array directly, so no conditional branches are needed in the component.

### Used by

- `src/hud/settings/SettingsAppearance.tsx` - filters the slider list for the selected mode

## MODE_HAS_SHAPE

```ts
export const MODE_HAS_SHAPE: Record<ModeKey, boolean>
```

Whether the cam-shape picker (circle / rounded / rect) is visible for a given mode. `screen_only` returns `false` because it has no camera overlay. The UI gates the shape row on this value.

### Used by

- `src/hud/settings/SettingsAppearance.tsx` - conditionally renders the shape picker row

## MODE_HAS_CORNER

```ts
export const MODE_HAS_CORNER: Record<ModeKey, boolean>
```

Whether the cam-corner picker (BL / BR / TL / TR) is visible. Only `screen` mode returns `true` because the corner concept applies when a small bubble camera is anchored inside a full-screen panel. Modes where the camera fills the frame have no meaningful corner to pick.

### Used by

- `src/hud/settings/SettingsAppearance.tsx` - conditionally renders the corner picker row

## SHAPES

```ts
export const SHAPES: [CamShape, string][] = [["circle", "Circle"], ["rounded", "Rounded"], ["rect", "Rect"]]
```

Ordered `[value, label]` pairs for the shape toggle buttons. Values match the `CamShape` union from `settings.ts`.

## CORNERS

```ts
export const CORNERS: [CamCorner, string][] = [["bottom_left", "BL"], ["bottom_right", "BR"], ["top_left", "TL"], ["top_right", "TR"]]
```

Ordered `[value, label]` pairs for the corner toggle buttons. Short labels keep the toggle compact in the panel.

## ASPECTS

```ts
export const ASPECTS: [CamAspect, string][] = [["square", "Square"], ["wide", "16:9"]]
```

Ordered `[value, label]` pairs for the webcam-PiP aspect-ratio picker. Values match the `CamAspect` union from `settings.ts`.

### Used by

- `src/editor/panels/CameraPanel.tsx` - renders the Aspect picker

## RING_WIDTH_SLIDER

```ts
export const RING_WIDTH_SLIDER: Spec = { label: "Ring width", min: 0.005, max: 0.2, step: 0.005 }
```

Slider range for the webcam ring/border width - same fraction-of-min-side units as `cam_radius`, so the numeric range is comparable.

### Used by

- `src/editor/panels/CameraRingField.tsx` - drives the ring-width slider bounds

## DEFAULT_RING

```ts
export const DEFAULT_RING = { width: 0.03, color: [255, 255, 255] as [number, number, number] }
```

The `CamRing` value written when the ring switch is toggled on (from `null`). *Why a fixed starting point:* gives the user a visible, sensible ring immediately instead of a 0-width or invisible one.

### Used by

- `src/editor/panels/CameraRingField.tsx` - the switch's on-value

## pct

```ts
export const pct = (v: number) => `${Math.round(v * 100)}%`
```

Converts a normalized fraction to a rounded percentage string for slider value labels.

### Inputs

- `v: number` - a fraction in `[0, 1]`. *Why a fraction:* all `ModeAppearance` fields are stored as normalized fractions; converting at display time avoids coupling storage format to UI.

### Returns

`string` of the form `"N%"`, e.g. `pct(0.1944)` returns `"19%"`.

## DEFAULT_APPEARANCE

```ts
export const DEFAULT_APPEARANCE: AppearanceSettings
```

Factory defaults for all five modes, shipped as code rather than a config file so settings initialization can reference this constant without a Tauri round-trip. Two internal presets drive the five modes:

- `bubble` - small 19% circular camera anchored bottom-left, used for `screen` and `screen_only`. Includes `cam_aspect: "square"` and `cam_ring: null`, matching the Rust `ModeAppearance::default()` byte-for-byte.
- `big` - 89% rounded camera filling most of the frame, spread from `bubble` with `cam_size` and `cam_shape` overridden, used for `camera`, `camera_only`, and `presenter`

### Used by

- `src/hud/settings/SettingsAppearance.tsx` - reset-to-defaults button
- `src/hud/Hud.tsx` - initial settings hydration fallback
- `src/editor/panels/CameraPanel.tsx` - reset-to-defaults button (screen mode only)
