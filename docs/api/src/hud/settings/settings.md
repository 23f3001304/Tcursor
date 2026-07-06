# src/hud/settings/settings.ts

Canonical TypeScript type definitions for all TCursor settings. No runtime code - every export is a type alias or interface. This file is the single source of truth that settings panels, IPC calls, and the Rust backend all converge on.

## CamShape

```ts
export type CamShape = "circle" | "rounded" | "rect";
```

Shape of the webcam overlay mask. `"circle"` clips to a perfect circle; `"rounded"` applies `cam_radius` as a corner radius; `"rect"` is a sharp rectangle.

### Used by

- `src/hud/settings/settings.ts` - `ModeAppearance.cam_shape`
- `src/hud/preferences/appearanceFields.ts` - `SHAPES` constant, `MODE_HAS_SHAPE` map
- `src/hud/settings/SettingsAppearance.tsx` - renders the shape picker

## CamCorner

```ts
export type CamCorner = "bottom_left" | "bottom_right" | "top_left" | "top_right";
```

Which corner of the screen panel anchors the floating camera bubble. Only meaningful when the recording mode shows a small overlay on top of a screen panel.

### Used by

- `src/hud/settings/settings.ts` - `ModeAppearance.cam_corner`
- `src/hud/preferences/appearanceFields.ts` - `CORNERS` constant, `MODE_HAS_CORNER` map
- `src/hud/settings/SettingsAppearance.tsx` - renders the corner picker

## ModeAppearance

```ts
export interface ModeAppearance {
  pad: number;
  screen_size: number;
  screen_radius: number;
  cam_size: number;
  cam_shape: CamShape;
  cam_radius: number;
  cam_corner: CamCorner;
  cam_margin_x: number;
  cam_margin_y: number;
}
```

All visual layout parameters for one recording mode. All numeric fields are normalized fractions (0-1) of the output dimension unless otherwise specified.

- `pad` - padding around the composite frame. *Why:* creates visual breathing room between the frame edge and the canvas boundary.
- `screen_size` - fraction of the output that the screen panel occupies (0.6-1.0). *Why:* in split-view modes the screen competes with the camera for canvas space.
- `screen_radius` - corner roundness of the screen panel (0-0.05). *Why:* modern broadcast aesthetics use rounded corners on screen panels.
- `cam_size` - fraction of the output that the camera overlay occupies (0.08-1.0). *Why:* ranges from a small bubble (bubble preset ~0.19) to a large frame fill (big preset ~0.89).
- `cam_shape: CamShape` - mask shape applied to the camera overlay.
- `cam_radius` - corner radius when `cam_shape` is `"rounded"` (0-0.5). *Why:* 0.5 produces a near-circle; lower values give soft-rounded rectangles.
- `cam_corner: CamCorner` - which screen corner anchors the bubble camera when mode is `"screen"`.
- `cam_margin_x` / `cam_margin_y` - gap between the camera bubble and the frame edge (0-0.1 each). *Why:* prevents the camera from being flush against the edge, which looks cramped.

### Used by

- `src/hud/settings/settings.ts` - `AppearanceSettings` holds one per mode
- `src/hud/preferences/appearanceFields.ts` - `DEFAULT_APPEARANCE` typed as `AppearanceSettings`
- `src/hud/settings/SettingsAppearance.tsx` - reads and mutates per-mode appearance
- `src/hud/components/LayoutPreview.tsx` - renders a thumbnail of the current layout

## AppearanceSettings

```ts
export interface AppearanceSettings {
  screen: ModeAppearance;
  screen_only: ModeAppearance;
  camera: ModeAppearance;
  camera_only: ModeAppearance;
  presenter: ModeAppearance;
}
```

Holds one `ModeAppearance` for each of the five layout modes. The keys match `ModeKey` from `appearanceFields.ts`.

### Used by

- `src/hud/settings/settings.ts` - `Settings.appearance`
- `src/hud/preferences/appearanceFields.ts` - `DEFAULT_APPEARANCE`
- `src/hud/settings/SettingsAppearance.tsx` - indexed by the active `ModeKey`

## ThemeMode

```ts
export type ThemeMode = "light" | "dark" | "system";
```

Controls the HUD color scheme. `"system"` defers to the OS `prefers-color-scheme` media query at runtime.

### Used by

- `src/hud/settings/settings.ts` - `InterfaceSettings.theme`
- `src/hud/preferences/applyTheme.ts` - theme resolution logic
- `src/hud/Hud.tsx` - re-applies theme on settings change and on OS media-query change

## InterfaceSettings

```ts
export interface InterfaceSettings { theme: ThemeMode; accent: [number, number, number] }
```

General UI appearance settings.

- `theme: ThemeMode` - color scheme selection.
- `accent: [number, number, number]` - RGB triplet (0-255) for the accent color applied as `--accent`. *Why a tuple:* compact JSON representation; formatted to `rgb()` at display time by `applyTheme`.

### Used by

- `src/hud/settings/settings.ts` - `Settings.ui`
- `src/hud/preferences/applyTheme.ts` - consumes both fields
- `src/hud/settings/SettingsInterface.tsx` - renders theme and accent pickers

## CursorStyle

```ts
export type CursorStyle = "system" | "enhanced" | "hidden";
```

Controls how the cursor is rendered in the recording. `"system"` uses the OS cursor sprite; `"enhanced"` uses TCursor's custom sprite pipeline; `"hidden"` suppresses the cursor entirely.

### Used by

- `src/hud/settings/settings.ts` - `CursorSettings.style`
- `src/hud/settings/SettingsCursor.tsx` - renders the style picker

## CursorSettings

```ts
export interface CursorSettings {
  style: CursorStyle;
  size: number;
  motion_blur: number;
  click_bounce: boolean;
  bounce_intensity: number;
}
```

Cursor rendering and animation parameters.

- `style: CursorStyle` - rendering mode.
- `size: number` - scale multiplier for the cursor sprite.
- `motion_blur: number` - strength of the motion-blur trail (0 = off).
- `click_bounce: boolean` - whether a spring-bounce animation plays on click.
- `bounce_intensity: number` - magnitude of the bounce when `click_bounce` is true.

### Used by

- `src/hud/settings/settings.ts` - `Settings.cursor`
- `src/hud/settings/SettingsCursor.tsx` - renders all cursor controls

## ClickFxStyle

```ts
export type ClickFxStyle = "none" | "ripple" | "pulse" | "glow" | "shockwave" | "particles" | "neon";
```

Which click-effect animation plays at each click site.

### Used by

- `src/hud/settings/settings.ts` - `ClickFxSettings.style`
- `src/hud/settings/SettingsClickFx.tsx` - renders the style picker

## SpotlightMode

```ts
export type SpotlightMode = "classic" | "blur" | "halo" | "breathing" | "nebula" | "vignette";
```

Rendering variant for the spotlight dimmer overlay.

### Used by

- `src/hud/settings/settings.ts` - `ClickFxSettings.spotlight_mode`
- `src/hud/settings/SettingsClickFx.tsx` - renders the spotlight-mode picker

## VideoFxMode

```ts
export type VideoFxMode = "nebulawash" | "cinematicdim" | "screenfocus" | "colorpop";
```

Full-screen video effect overlay applied to the recording.

### Used by

- `src/hud/settings/settings.ts` - `ClickFxSettings.video_fx_mode`
- `src/hud/settings/SettingsClickFx.tsx` - renders the video-fx picker

## ZoomSettings

```ts
export interface ZoomSettings {
  enabled: boolean;
  target_scale: number;
  hold_ms: number;
  smoothness: number;
  clicks: number;
  camera_shrink: boolean;
  camera_shrink_min: number;
  smart_hold: boolean;
  smart_follow: boolean;
}
```

Configuration for the auto-zoom feature.

- `enabled` - whether auto-zoom is active.
- `target_scale` - zoom magnification factor.
- `hold_ms` - how long the zoom holds after the last trigger activity before releasing.
- `smoothness` - easing strength for the zoom transition.
- `clicks` - number of clicks required to trigger a zoom burst.
- `camera_shrink` - whether to shrink the camera overlay while zoomed in.
- `camera_shrink_min` - minimum camera size when shrunk (fraction).
- `smart_hold` - whether typing keystrokes extend the zoom hold.
- `smart_follow` - whether the zoom anchor follows the cursor during a hold.

### Used by

- `src/hud/settings/settings.ts` - `Settings.zoom`
- `src/hud/settings/SettingsZoom.tsx` - renders all zoom controls
- `src/lib/ipc.ts` - serialized into Tauri commands

## ClickFxSettings

```ts
export interface ClickFxSettings {
  enabled: boolean;
  style: ClickFxStyle;
  color: [number, number, number];
  intensity: number;
  captions: boolean;
  spotlight: boolean;
  spotlight_dim: number;
  spotlight_radius: number;
  spotlight_feather: number;
  spotlight_mode: SpotlightMode;
  spotlight_tint: [number, number, number];
  video_fx_mode: VideoFxMode;
}
```

Configuration for click effects and the spotlight overlay.

- `enabled` - master on/off for click effects.
- `style: ClickFxStyle` - animation variant.
- `color` - RGB triplet for the effect color.
- `intensity` - effect strength multiplier.
- `captions` - whether click-site captions are rendered.
- `spotlight` - whether the spotlight dim overlay activates on click.
- `spotlight_dim` - opacity of the spotlight darkening (0-1).
- `spotlight_radius` - radius of the bright spotlight circle.
- `spotlight_feather` - softness of the spotlight edge.
- `spotlight_mode: SpotlightMode` - rendering variant for the spotlight.
- `spotlight_tint` - RGB tint applied inside the spotlight circle.
- `video_fx_mode: VideoFxMode` - full-screen overlay mode.

### Used by

- `src/hud/settings/settings.ts` - `Settings.clickfx`
- `src/hud/settings/SettingsClickFx.tsx` - renders all click-fx and spotlight controls

## HotkeySettings

```ts
export interface HotkeySettings {
  zoom_hold: string;
  layout_screen: string;
  layout_camera: string;
  layout_presenter: string;
  layout_screen_only: string;
  layout_camera_only: string;
  spotlight_hold: string;
  video_fx_hold: string;
}
```

Maps each hotkey action to its key-binding string. Each field is the key combo shown in the hotkey editor and registered with Tauri's global shortcut system.

### Used by

- `src/hud/settings/settings.ts` - `Settings.hotkeys`
- `src/hud/settings/SettingsHotkeys.tsx` - renders the hotkey binding editor

## Settings

```ts
export interface Settings {
  zoom: ZoomSettings;
  clickfx: ClickFxSettings;
  hotkeys: HotkeySettings;
  appearance: AppearanceSettings;
  cursor: CursorSettings;
  ui: InterfaceSettings;
  audio_offset_ms: number;
}
```

Top-level interface aggregating all settings groups. Serialized to/from JSON by Tauri's `load_settings` and `save_settings` commands.

- `zoom: ZoomSettings` - auto-zoom configuration.
- `clickfx: ClickFxSettings` - click effects and spotlight.
- `hotkeys: HotkeySettings` - global shortcut bindings.
- `appearance: AppearanceSettings` - per-mode visual layout.
- `cursor: CursorSettings` - cursor rendering.
- `ui: InterfaceSettings` - theme and accent.
- `audio_offset_ms: number` - A/V sync correction in milliseconds; positive shifts audio later relative to video.

### Used by

- `src/hud/Hud.tsx` - top-level settings state
- `src/lib/ipc.ts` - `loadSettings` and `saveSettings` IPC wrappers
- `src/lib/edit.ts` - passed to the edit pipeline
