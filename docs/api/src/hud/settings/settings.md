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
- `src/editor/panels/camera/CameraPanel.tsx` - renders the Aspect picker

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
- `src/editor/panels/camera/CameraRingField.tsx` - renders the ring on/off switch, width slider, and color swatches

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
- `src/editor/panels/camera/CameraPanel.tsx` - reads and mutates `cam_aspect`/`cam_ring` for the "screen" mode's webcam PiP
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
- `src/editor/panels/layout/LayoutsPanel.tsx` - indexed by the picked layout, and snapshotted whole into a `LayoutPreset`

## LayoutPreset

```ts
export interface LayoutPreset { id: string; name: string; appearance: AppearanceSettings }
```

One saved "look" - a name plus a snapshot of **all five** layouts' appearance. Wire form of Rust's `LayoutPreset` (`src-tauri/src/settings/model.rs`).

- `id: string` - opaque and stable (`lp1`, `lp2`, ... - `nextPresetId` in `src/editor/panels/layout/layoutPresets.ts`). Renaming changes `name` only, so a row keeps its identity across a rename.
- `name: string` - what the user typed, trimmed. Unique case-insensitively across the list AND the built-in "Default" row (`presetNameError`), and capped at 40 characters so a row never has to ellipsise at 320px.
- `appearance: AppearanceSettings` - all five layouts at once. A look is deliberately not per-layout: a coherent look is the relationship BETWEEN the layouts a recording cuts among, and applying one is therefore a single write of `settings.appearance`.

Global by design: presets live in the app config, not in a recording's `edit.json`, which is what lets the Layouts panel apply the same look to a project recorded months later.

### Used by

- `src/hud/settings/settings.ts` - `Settings.layout_presets`
- `src/editor/panels/layout/layoutPresets.ts` - `BUILTIN_PRESET` and the four list helpers
- `src/editor/panels/layout/LayoutPresetList.tsx` - one row per entry

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
export interface InterfaceSettings { theme: ThemeMode; accent: [number, number, number]; animated_brand: boolean; interface_effects: boolean; ai_choreography: boolean }
```

General UI appearance settings.

- `theme: ThemeMode` - color scheme selection.
- `accent: [number, number, number]` - RGB triplet (0-255) for the accent color applied as `--accent`. *Why a tuple:* compact JSON representation; formatted to `rgb()` at display time by `applyTheme`.
- `animated_brand: boolean` (Task 39) - the living-brand feel knob: whether `TcursorMark` (`src/shared/brand/TcursorMark.tsx`) flows/pulses for its recording/exporting/directing states, in the HUD titlebar and the editor's `TopBar`. `false` and the OS `prefers-reduced-motion` both fall the mark back to its static idle rendering (the setting and the OS preference are independent gates - either alone is enough to disable the animation).
- `interface_effects: boolean` (micro-interaction pass, 2026-09-14) - the editor's own micro-interactions: the click ripple under every pointerdown in the chrome, and the magnetic pull the Play button and the Trim pills exert on a nearby pointer (`src/editor/effects/`). Mirrors Rust `InterfaceSettings::interface_effects`, which defaults it `true` so a config written before the field existed still loads with the feature on. Off, the ripple overlay unmounts entirely and the magnetic hook adds no listener; `prefers-reduced-motion` is an independent gate, as with `animated_brand`. Nothing here reaches the **export** - the recording's own click effects are `ClickFxSettings`.
- `ai_choreography: boolean` (M4 T5) - the AI Director's pointer replay after the review sheet's Apply: the editor's fake pointer walks the applied edits (`src/editor/director/useAiRun.ts`). Mirrors Rust `InterfaceSettings::ai_choreography`, which defaults it `false`: the edits are already applied by then, this only performs them. Read doc-scoped (`doc.settings.ui`) and edited in the editor's Interface section (`src/editor/shell/settings/InterfaceSection.tsx`), not the HUD's.

### Used by

- `src/hud/settings/settings.ts` - `Settings.ui`
- `src/editor/director/useAiRun.ts` - reads `doc.settings.ui.ai_choreography` after an apply to decide whether the pointer replay runs
- `src/editor/shell/settings/InterfaceSection.tsx` - the "Replay applied edits with the pointer" switch and `DEFAULT_INTERFACE_RESET`
- `src/hud/preferences/applyTheme.ts` - consumes `theme`/`accent`
- `src/hud/settings/SettingsInterface.tsx` - renders theme, accent, the animated-brand switch (Task 39) and the interface-effects switch
- `src/editor/effects/InterfaceEffects.tsx` - reads `interface_effects` through `getSettings()` on mount and on every window focus
- `src/hud/Hud.tsx` - reads `animated_brand` (mirrored into its own `animatedBrand` state) to gate the titlebar mark
- `src/editor/Editor.tsx` - reads `doc.settings.ui.animated_brand` to gate `TopBar`'s `brandState`

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
  tilt: number;
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
- `tilt: number` - 0..1 motion lean (default `0.35`): how far a fast cursor tips into its own travel, and overshoots once coming back upright when it stops. Scales the 6-degree cap; `0` switches the filter off. Mirrors Rust `CursorSettings::tilt` (`src-tauri/src/settings/cursor.rs`); the live preview computes the same angle through `src/editor/stage/cursor/cursorTilt.ts`, pinned against Rust's own five instants. *Why a separate knob from `motion_blur`:* the trail says where the cursor has been, the lean says how hard it is being thrown - a user who wants one rarely wants both at full strength.
- `click_bounce: boolean` - whether a spring-bounce animation plays on click.
- `bounce_intensity: number` - magnitude of the bounce when `click_bounce` is true.
- `pack: string` - selected cursor sprite pack id. `"default"` is the built-in set (byte-identical to before this field existed); any other value is an imported pack's id (`CursorPackInfo.id` from `listCursorPacks`/`importCursorPack` in `src/shared/ipc.ts`).

### Used by

- `src/hud/settings/settings.ts` - `Settings.cursor`
- `src/hud/settings/SettingsCursor.tsx` - renders style/size/blur/tilt/bounce controls (not `pack`, `smoothness`, or `path_idealize` - those are editor-only, see `CursorPanel`)
- `src/editor/panels/cursor/CursorPanel.tsx` - renders the pack picker + import button, plus the `smoothness`/`path_idealize` sliders, in addition to the same style/size/blur/tilt/bounce controls

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
- `src/shared/edit.ts` - re-exported for `Zoom.cam_action` (the per-zoom override) and the `set_zoom_cam_action` `EditOp`.
- `src/editor/stage/camera/camZoomAction.ts` - `resolvedCamDefault`/`resolveCamAction` resolve which action applies at a given time; `applyCamZoomAction`/`camZoomAlpha` turn it into the geometry/alpha the preview draws.

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
  camera_smoothing_ms: number;
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
- `camera_smoothing_ms` - opt-in critically-damped smoothing on the auto-zoom CAMERA path, wire form of Rust `ZoomSettings::camera_smoothing_ms` (`settings/model.md`), forwarded verbatim to `ZoomConfig::smoothing_ms` (`export/types.md`, `export/camera/smoothing.md`). `0` = off (bit-identical export); UI range is 0-400ms in steps of 10. *Why a separate field from `smoothness` above:* `smoothness` is the camera's follow damping, not this post-pass filter - and it is also unrelated to `CursorSettings.smoothness` (the CURSOR low-pass, a different concern entirely).

### Used by

- `src/hud/settings/settings.ts` - `Settings.zoom`
- `src/hud/settings/SettingsZoom.tsx` - renders all zoom controls (not `cam_zoom_default` - not yet exposed by any settings UI)
- `src/shared/ipc.ts` - serialized into Tauri commands
- `src/editor/stage/camera/camZoomAction.ts` - `resolvedCamDefault` reads `cam_zoom_default` to compute the global webcam-during-zoom behavior
- `src/editor/shell/settings/ZoomDefaultsSection.tsx` - renders the editor's own copy of the zoom-defaults controls, including `camera_smoothing_ms`

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
- `src/editor/hooks/stage/useCompositeLoop.ts` - reads `spotlight_dim_camera` to thread `dimCamera` into `requestFxOverlay`

## CaptionPos

```ts
export type CaptionPos = "bottom" | "top";
```

Where the caption band sits on the output frame - mirrors Rust `settings::captions::CaptionPos`.

### Used by

- `src/hud/settings/settings.ts` - `CaptionStyle.position`

## CaptionSize

```ts
export type CaptionSize = "s" | "m" | "l";
```

Caption font-size rung - mirrors Rust `settings::captions::CaptionSize`. Rust's `CaptionSize::height_frac` resolves each rung to a fraction of output height (0.030 / 0.038 / 0.048).

### Used by

- `src/hud/settings/settings.ts` - `CaptionStyle.size`

## CaptionStyle

```ts
export interface CaptionStyle {
  enabled: boolean; position: CaptionPos; size: CaptionSize; pill: boolean; highlight: boolean;
  model: string; language: string;
}
```

The caption look plus the two ASR inputs the panel edits alongside it - mirrors Rust `settings::captions::CaptionStyle`. Distinct from `ClickFxSettings.captions`, the OLD hotkey-chord toggle, which keeps its own name and meaning.

- `enabled` - master on/off for the caption overlay.
- `position: CaptionPos` - bottom or top band.
- `size: CaptionSize` - font-size rung.
- `pill` - whether a background pill is drawn behind the text.
- `highlight` - whether the active word is highlighted as it is spoken (needs word-level timings on the caption).
- `model` - the whisper model id to transcribe with (e.g. `"base.en"`, `"small.en"`).
- `language` - ASR language hint (`"en"` or `"auto"` with a multilingual model).

**Not here:** the accent color. The renderer reads `InterfaceSettings.accent`, so there is exactly one accent in the doc.

### Used by

- `src/hud/settings/settings.ts` - `Settings.captions`

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
export type BackgroundKind = "mesh" | "solid" | "gradient" | "image" | "video";
```

Which of `BackgroundSettings`' fields the renderer uses - mirrors Rust `settings::background::BackgroundKind`. `"mesh"` (the default) is a bundled wallpaper image, picked by `BackgroundSettings.mesh`; `"solid"`/`"gradient"` are real user-chosen colors; `"image"`/`"video"` render the user's own imported file, named by `BackgroundSettings.asset`. A GIF is a `"video"` - one decode path for both in the export; only the preview tells them apart, and it does that by extension (`stage/canvas/gifFrames.ts`).

### Used by

- `src/hud/settings/settings.ts` - `BackgroundSettings.kind`
- `src/editor/panels/background/BackgroundPanel.tsx` - the Background Type selector (Wallpapers / Color / Gradient tabs map to `mesh`/`solid`/`gradient`; `image`/`video` come from the asset card at the end of the Wallpapers tab and share that tab)
- `src/editor/stage/canvas/stageBg.ts` - decides whether the preview draws the backend's still PNG or the moving asset itself

## BackgroundSettings

```ts
export interface BackgroundSettings {
  kind: BackgroundKind;
  solid: [number, number, number];
  gradient_from: [number, number, number];
  gradient_to: [number, number, number];
  gradient_angle_deg: number;
  blur: number;
  mesh: string;
  gradient_mid?: [number, number, number] | null;
  asset?: string | null;
  dim: number;
}
```

The recording's background, behind the screen/webcam panels. Mirrors the Rust `settings::background::BackgroundSettings` byte-for-byte.

- `kind: BackgroundKind` - which of the fields below the renderer actually uses.
- `solid: [number, number, number]` - RGB triplet used when `kind` is `"solid"`.
- `gradient_from` / `gradient_to: [number, number, number]` - the gradient's end stops, used when `kind` is `"gradient"`.
- `gradient_angle_deg: number` - gradient direction in degrees, same convention as CSS `linear-gradient()`.
- `blur: number` - 0..1 softness applied once to the STATIC background buffer (cheap - rebuilt once per export/preview, not per frame). `0` = off (today's behavior). Because it is a one-off pass it reaches a video background's first frame only, which is why `BackgroundPanel` hides its slider while `kind` is `"video"` instead of showing a control that does nothing.
- `asset?: string | null` - the user's imported background file, RELATIVE to the project folder (`background/<file>`, forward-slashed), used by `"image"`/`"video"`. Never absolute: a project folder is copyable, and an absolute path would break the moment it was. Kept when the user switches back to a wallpaper, so re-selecting the asset card needs no re-import; only the card's Remove deletes the file and clears this.
- `dim: number` - 0..0.8 black overlay over whichever background actually has pixels (wallpaper, image or video). Applied exactly once by whichever side owns the pixels: Rust (`background::apply_dim`) for everything the backend rasterises and for each streamed video frame, `stage/canvas/stageBg.ts` for the preview's own video/GIF draw.
- `mesh: string` - which bundled wallpaper `kind: "mesh"` renders (`settings::wallpapers::WALLPAPERS` id). EMPTY is the legacy "Classic" `bg.jpg` and is what every project saved before the wallpaper library loads as, so those keep rendering byte-identically.
- `gradient_mid?: [number, number, number] | null` - optional middle stop, sitting at the ramp's midpoint. Absent or `null` is the two-stop ramp, unchanged; Rust omits the key entirely when unset.

### Used by

- `src/hud/settings/settings.ts` - `Settings.background`
- `src/editor/panels/background/BackgroundPanel.tsx` - reads and patches every field; its colour swatches (`backgroundPresets.ts`) ship in the same plain-RGB shape so a swatch always renders identically to what gets applied, and its wallpaper/gradient tiles are rendered by the backend itself (`backgroundThumbs`)
- `src/editor/hooks/doc/useEditorData.ts` - refetches `previewBg` whenever `JSON.stringify(doc?.settings.background)` changes, since a background edit is the only kind of change that alters what the backend's background render returns

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
  layout_presets: LayoutPreset[];
  captions: CaptionStyle;
  motion: MotionSettings;
  grade: GradeSettings;
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
- `layout_presets: LayoutPreset[]` - the user's saved layout looks, newest last. Serde-defaulted on the Rust side, so a config written before presets existed arrives as `[]`. The editor's Layouts panel is the only reader and writer; it read-modify-writes the whole `Settings` so every other field here survives a preset edit.
- `captions: CaptionStyle` - the caption look plus its ASR inputs. Distinct from `clickfx.captions`, the OLD hotkey-chord toggle, which keeps its own name and meaning.

- `motion: MotionSettings` - the project's motion language. Serde-defaulted on the Rust side, so a config written before it existed arrives as Soft.
- `grade: GradeSettings` - the colour grade: a preset name plus the three absolute numbers it seeds (`exposure`, `contrast`, `vignette`). Serde-defaulted on the Rust side, so a config written before it existed arrives at the identity and renders unchanged.

**Note on `EditDoc.settings`.** `Settings` is also the shape of a recording's `edit.json` settings block, so `layout_presets` technically rides along in every project file (serialized as `[]` unless a doc was seeded from a config that had looks in it). Nothing reads it from there: the panel always asks `get_settings` for the library, precisely so a look is global and a project carries only the look it was GIVEN, not the library it came from.

### Used by

- `src/hud/Hud.tsx` - top-level settings state
- `src/shared/ipc.ts` - `getSettings` and `setSettings` IPC wrappers
- `src/shared/edit.ts` - passed to the edit pipeline (`EditDoc.settings`)
- `src/editor/panels/layout/LayoutsPanel.tsx` - reads and writes the whole object for `layout_presets` and the global `appearance` default

## MotionSettings

```ts
export interface MotionSettings { preset: string; easing: string; easing_out: string }
```

The project's ONE motion language (M3) - mirrors Rust `settings::motion::MotionSettings`. Every newly added zoom, layout segment and camera keyframe inherits this curve pair (`edit::ops::motion`), and `{ op: "apply_motion_default" }` stamps it onto the ones already placed.

- `preset: string` - the name the editor showed when the pair was written (`"soft"`, `"snappy"`, `"cinematic"`, `"mechanical"`, `"bouncy"`). Provenance for the UI only; nothing in the render path reads it, and `MotionSection`'s picker derives what to show from the STRINGS via `presetOf`, so a curve dragged off a preset in an inspector honestly reads as Custom.
- `easing: string` - the ramp INTO a region: a zoom-in, a layout segment's entry fade, a camera move's blend from the previous keyframe.
- `easing_out: string` - the ramp OUT of one: a zoom-out, a layout segment's exit fade. A camera move has no exit ramp of its own and reads `easing` only.

The default is Soft, and **Soft is the bare word `"smooth"`**, not the equivalent `keys(...)` curve - see `docs/api/src-tauri/src/settings/motion.md` for why (every existing region reads back as Soft, and the shipped camera trajectory stays bit-identical).

### Used by

- `src/hud/settings/settings.ts` (`Settings.motion`)
- `src/editor/shell/settings/MotionSection.tsx` - the only writer
- `src/editor/motion/presets.ts` - `presetOf` / `presetPatch` read and produce these two strings

## CursorBackStyle

```ts
export type CursorBackStyle = "none" | "glass";
```

The glass shape drawn BEHIND the cursor, whatever pack it comes from - the wire mirror of Rust `settings::cursor::CursorBack`. `"none"` is the original look; `"glass"` adds a refracting disc that morphs by cursor kind (a horizontal pill over text, stretching into a selection bar while the left button is held there).

Independent of the pack's own `material`: a plain pack can have a glass back, and a glass pack can have none. Carried on `CursorSettings.back`, edited by `CursorPanel.tsx` (a `Picker`) and `SettingsCursor.tsx` (a segment), and read by the preview through `DrawCursor.back`.

## GradePreset

```ts
export type GradePreset =
  | "none"
  | "cinematic"
  | "noir"
  | "vintage"
  | "frost"
  | "golden"
  | "midnight"
  | "vivid"
  | "dreamy";
```

The nine names of Rust `settings::grade::GradePreset` - the wire mirror, one string per look (see `docs/api/src-tauri/src/settings/grade.md`). `"none"` is the default and the only one with no visual effect.

### Used by

- `src/hud/settings/settings.ts` (`GradeSettings.preset`)
- `src-tauri/src/settings/grade.rs` (`GradePreset`) - the Rust source of truth

## GradeSettings

```ts
export interface GradeSettings {
  preset: GradePreset;
  exposure: number;
  contrast: number;
  vignette: number;
}
```

Mirrors Rust `settings::grade::GradeSettings` field for field (see `docs/api/src-tauri/src/settings/grade.md` for the full story: the three ABSOLUTE numbers a preset writes, the identity default, and why that makes every document byte-identical until a look is chosen).

- `preset: GradePreset` - the last look picked. Default `"none"`.
- `exposure: number` - stops, `-2.0` to `+2.0`. Default `0`.
- `contrast: number` - multiplier about a 0.5 pivot, `0.5` to `1.8`. Default `1`.
- `vignette: number` - `0` to `1`, how dark the corners go. Default `0`.

### Used by

- `src/hud/settings/settings.ts` (`Settings.grade`)

**`CaptionStyle` grew seven fields on 2026-09-15** (`font_pct`, `text_color`, `highlight_color` or `null` for the interface accent, `pill_color`, `pill_alpha`, `animation`, `animation_ms`) and the `CaptionAnim` union (`none` | `fade` | `rise` | `pop` | `words`), mirroring `settings/captions.rs` field for field; the defaults live in `DEFAULT_CAPTION_STYLE` (`editor/panels/captions/CaptionStyleControls.tsx`) and reproduce the previous look.
