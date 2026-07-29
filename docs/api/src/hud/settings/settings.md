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

## CamAspect

```ts
export type CamAspect = "square" | "wide";
```

Aspect ratio of the webcam PiP bubble (Screen/ScreenOnly modes only - big-camera modes always stay square). `"square"` is 1:1 (today's only shape, and the default); `"wide"` is 16:9, widening the panel while `cam_size` stays the HEIGHT basis.

### Used by

- `src/hud/settings/settings.ts` - `ModeAppearance.cam_aspect`
- `src/hud/preferences/appearanceFields.ts` - `ASPECTS` constant
- `src/editor/panels/CameraPanel.tsx` - renders the Aspect picker

## CamRing

```ts
export interface CamRing { width: number; color: [number, number, number] }
```

Optional colored ring/border drawn just inside the webcam panel edge. Mirrors the Rust `CamRing` (`src-tauri/src/settings/appearance.rs`) byte-for-byte.

- `width: number` - fraction of the panel's min side (same units as `cam_radius`).
- `color: [number, number, number]` - RGB triplet (0-255).

### Used by

- `src/hud/settings/settings.ts` - `ModeAppearance.cam_ring` (`null` = no ring)
- `src/hud/preferences/appearanceFields.ts` - `DEFAULT_RING` constant
- `src/editor/panels/CameraRingField.tsx` - renders the ring on/off switch, width slider, and color swatches

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
  cam_aspect: CamAspect;
  cam_ring: CamRing | null;
}
```

All visual layout parameters for one recording mode. All numeric fields are normalized fractions (0-1) of the output dimension unless otherwise specified.

- `pad` - padding around the composite frame. *Why:* creates visual breathing room between the frame edge and the canvas boundary.
- `screen_size` - fraction of the output that the screen panel occupies (0.6-1.0). *Why:* in split-view modes the screen competes with the camera for canvas space.
- `screen_radius` - corner roundness of the screen panel (0-0.05). *Why:* modern broadcast aesthetics use rounded corners on screen panels.
- `cam_size` - fraction of the output that the camera overlay occupies as its HEIGHT (0.08-1.0). *Why:* ranges from a small bubble (bubble preset ~0.19) to a large frame fill (big preset ~0.89); `cam_aspect` derives the width from this when `"wide"`.
- `cam_shape: CamShape` - mask shape applied to the camera overlay.
- `cam_radius` - corner radius when `cam_shape` is `"rounded"` (0-0.5). *Why:* 0.5 produces a near-circle; lower values give soft-rounded rectangles.
- `cam_corner: CamCorner` - which screen corner anchors the bubble camera when mode is `"screen"`.
- `cam_margin_x` / `cam_margin_y` - gap between the camera bubble and the frame edge (0-0.1 each). *Why:* prevents the camera from being flush against the edge, which looks cramped.
- `cam_aspect: CamAspect` - PiP bubble width:height ratio. Default `"square"` (byte-identical to before this field existed).
- `cam_ring: CamRing | null` - optional colored border just inside the panel edge. Default `null` (no ring).

### Used by

- `src/hud/settings/settings.ts` - `AppearanceSettings` holds one per mode
- `src/hud/preferences/appearanceFields.ts` - `DEFAULT_APPEARANCE` typed as `AppearanceSettings`
- `src/hud/settings/SettingsAppearance.tsx` - reads and mutates per-mode appearance
- `src/editor/panels/CameraPanel.tsx` - reads and mutates `cam_aspect`/`cam_ring` for the "screen" mode's webcam PiP
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
  smoothness: number;
  path_idealize: number;
  motion_blur: number;
  click_bounce: boolean;
  bounce_intensity: number;
  pack: string;
}
```

Cursor rendering and animation parameters. Mirrors the Rust `CursorSettings` (`src-tauri/src/settings/model.rs`) byte-for-byte.

- `style: CursorStyle` - rendering mode.
- `size: number` - scale multiplier for the cursor sprite.
- `smoothness: number` - 0..1 strength of motion smoothing applied to the raw recorded cursor path (default `0.6`). *Why:* raw OS cursor samples can be jittery; smoothing trades a touch of positional lag for a calmer glide, independent of the stronger reshaping `path_idealize` does below.
- `path_idealize: number` - 0..1 strength of straightening wandering paths into clean eased strokes between clicks (`0` = raw path, the default; `1` = fully idealized). Mirrors Rust `CursorSettings::path_idealize`; see `Cursor::set_idealize` (`src-tauri/src/export/cursor/mod.rs`) for the anchor-easing mechanism. *Why a separate knob from `smoothness`:* smoothing damps jitter without changing the path's shape, while idealizing reshapes the path itself into deliberate strokes - part of the broader design direction of idealizing UI motion (cursor glide, agentic AI reveals) for a more premium, intentional feel, which needs its own strength dial rather than riding on the jitter-smoothing one.
- `motion_blur: number` - strength of the motion-blur trail (0 = off).
- `click_bounce: boolean` - whether a spring-bounce animation plays on click.
- `bounce_intensity: number` - magnitude of the bounce when `click_bounce` is true.
- `pack: string` - selected cursor sprite pack id. `"default"` is the built-in set (byte-identical to before this field existed); any other value is an imported pack's id (`CursorPackInfo.id` from `listCursorPacks`/`importCursorPack` in `src/lib/ipc.ts`).

### Used by

- `src/hud/settings/settings.ts` - `Settings.cursor`
- `src/hud/settings/SettingsCursor.tsx` - renders style/size/blur/bounce controls (not `pack`, `smoothness`, or `path_idealize` - those are editor-only, see `CursorPanel`)
- `src/editor/panels/CursorPanel.tsx` - renders the pack picker + import button, plus the `smoothness`/`path_idealize` sliders, in addition to the same style/size/blur/bounce controls

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

## CamZoomAction

```ts
export type CamZoomAction = { shrink: { to: number } } | "hide" | "stay";
```

What the webcam PiP does while a zoom is active. Wire form of Rust's `CamZoomAction` (`settings/model.rs`): serde's externally-tagged encoding gives `{ shrink: { to } }` for the struct variant and bare `"hide"`/`"stay"` for the unit ones.

- `{ shrink: { to: number } }` - shrinks the webcam panel toward size `to` (a 0..1 fraction) as the zoom deepens.
- `"hide"` - fades the webcam out entirely as the zoom deepens.
- `"stay"` - leaves the webcam panel's geometry untouched.

### Used by

- `src/hud/settings/settings.ts` - `ZoomSettings.cam_zoom_default` (the global default).
- `src/lib/edit.ts` - re-exported for `Zoom.cam_action` (the per-zoom override) and the `set_zoom_cam_action` `EditOp`.
- `src/editor/stage/camZoomAction.ts` - `resolvedCamDefault`/`resolveCamAction` resolve which action applies at a given time; `applyCamZoomAction`/`camZoomAlpha` turn it into the geometry/alpha the preview draws.

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
  cam_zoom_default?: CamZoomAction | null;
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
- `cam_zoom_default?: CamZoomAction | null` - the global default for what the webcam PiP does during a zoom (`resolvedCamDefault`), used whenever a zoom has no per-zoom `Zoom.cam_action` override. Absent/`null` falls back to the legacy `camera_shrink`/`camera_shrink_min` pair - `{shrink:{to:camera_shrink_min}}` when `camera_shrink` is on, else `"stay"` - so configs saved before this field existed resolve to exactly today's behavior.

### Used by

- `src/hud/settings/settings.ts` - `Settings.zoom`
- `src/hud/settings/SettingsZoom.tsx` - renders all zoom controls (not `cam_zoom_default` - not yet exposed by any settings UI)
- `src/lib/ipc.ts` - serialized into Tauri commands
- `src/editor/stage/camZoomAction.ts` - `resolvedCamDefault` reads `cam_zoom_default` to compute the global webcam-during-zoom behavior

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
  spotlight_dim_camera: boolean;
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
- `spotlight_dim_camera` - whether the spotlight dim also darkens the webcam PiP. Default `true` (today's behavior). Set `false` to keep the webcam fully lit while the rest of the frame still dims - mirrors the Rust `ClickFxSettings::spotlight_dim_camera` byte-for-byte; both the export (GPU + CPU) and the editor preview read it.

### Used by

- `src/hud/settings/settings.ts` - `Settings.clickfx`
- `src/hud/settings/SettingsClickFx.tsx` - renders all click-fx and spotlight controls (does not yet expose `spotlight_dim_camera` - only `src/editor/panels/EffectsPanel.tsx` does, as of this field's introduction)
- `src/editor/panels/EffectsPanel.tsx` - renders the "Dim webcam" toggle bound to `spotlight_dim_camera`
- `src/editor/hooks/useCompositeLoop.ts` - reads `spotlight_dim_camera` to thread `dimCamera` into `requestFxOverlay`

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

## BackgroundKind

```ts
export type BackgroundKind = "mesh" | "solid" | "gradient";
```

Which of `BackgroundSettings`' fields the renderer uses - mirrors Rust `settings::background::BackgroundKind`. `"mesh"` (the default) is today's bundled image background; `"solid"`/`"gradient"` are real user-chosen colors. Custom image/video backgrounds have no backend yet - `BackgroundPanel` flags those tabs as "coming soon" rather than wiring them to a `kind` that doesn't exist.

### Used by

- `src/hud/settings/settings.ts` - `BackgroundSettings.kind`
- `src/editor/panels/BackgroundPanel.tsx` - the Background Type selector (`default`/`color`/`gradient` tabs map to `mesh`/`solid`/`gradient`)

## BackgroundSettings

```ts
export interface BackgroundSettings {
  kind: BackgroundKind;
  solid: [number, number, number];
  gradient_from: [number, number, number];
  gradient_to: [number, number, number];
  gradient_angle_deg: number;
  blur: number;
}
```

The recording's background, behind the screen/webcam panels. Mirrors the Rust `settings::background::BackgroundSettings` byte-for-byte.

- `kind: BackgroundKind` - which of the fields below the renderer actually uses.
- `solid: [number, number, number]` - RGB triplet used when `kind` is `"solid"`.
- `gradient_from` / `gradient_to: [number, number, number]` - the two RGB stops used when `kind` is `"gradient"`.
- `gradient_angle_deg: number` - gradient direction in degrees, same convention as CSS `linear-gradient()`.
- `blur: number` - 0..1 softness applied once to the static background buffer (cheap - rebuilt once per export/preview, not per frame). `0` = off (today's behavior).

### Used by

- `src/hud/settings/settings.ts` - `Settings.background`
- `src/editor/panels/BackgroundPanel.tsx` - reads and patches every field; its fixed preset swatches (`backgroundPresets.ts`) ship in the same plain-RGB shape so a swatch always renders identically to what gets applied
- `src/editor/hooks/useEditorData.ts` - refetches `previewBg` whenever `JSON.stringify(doc?.settings.background)` changes, since a background edit is the only kind of change that alters what the backend's background render returns

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
  background: BackgroundSettings;
  audio_mic_volume: number;
  audio_sys_volume: number;
  ai_model: string;
}
```

Top-level interface aggregating all settings groups. Serialized to/from JSON by Tauri's `get_settings` and `set_settings` commands.

- `zoom: ZoomSettings` - auto-zoom configuration.
- `clickfx: ClickFxSettings` - click effects and spotlight.
- `hotkeys: HotkeySettings` - global shortcut bindings.
- `appearance: AppearanceSettings` - per-mode visual layout.
- `cursor: CursorSettings` - cursor rendering.
- `ui: InterfaceSettings` - theme and accent.
- `audio_offset_ms: number` - A/V sync correction in milliseconds; positive shifts audio later relative to video.
- `background: BackgroundSettings` - the recording's background (mesh/solid/gradient) and its blur.
- `audio_mic_volume` / `audio_sys_volume: number` - per-source playback gain (0..1) for the mixed preview/export audio, set by the editor's Audio panel.
- `ai_model: string` - the user's chosen Ollama model override for the AI director (`""` = no explicit choice; the backend picks an installed model itself). Set by `AiPanel`'s Engine picker.

### Used by

- `src/hud/Hud.tsx` - top-level settings state
- `src/lib/ipc.ts` - `getSettings` and `setSettings` IPC wrappers
- `src/lib/edit.ts` - passed to the edit pipeline (`EditDoc.settings`)
