# src-tauri/src/settings/model.rs

Defines the complete user-facing `Settings` tree: one top-level struct and all nested configuration structs and enums that are persisted to `config.json`. Every struct carries `#[serde(default)]` so that old config files gain new fields silently, enabling additive schema evolution with no migration code.

## ZoomSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ZoomSettings {
    pub enabled: bool,
    pub target_scale: f32,
    pub hold_ms: u32,
    pub smoothness: f32,
    pub clicks: u32,
    pub camera_shrink: bool,
    pub camera_shrink_min: f32,
    pub smart_hold: bool,
    pub smart_follow: bool,
}
```

User-facing knobs for the auto-zoom subsystem. Only the fields the UI exposes live here; internal algorithm constants remain in `ZoomConfig::default()` and are not surfaced to users.

Fields:

- `enabled: bool` - master on/off switch for auto-zoom. Default `true`. *Why:* lets power users disable auto-zoom entirely for recordings where it would be distracting.
- `target_scale: f32` - zoom magnification factor (e.g. `2.2` means the zoomed viewport is 1/2.2 of the screen). Default `2.2`. *Why:* a tuned value that fills the frame without cropping too aggressively on a 1080p screen.
- `hold_ms: u32` - time in milliseconds the zoom stays active after activity stops before easing back out. Default `2200`. *Why:* long enough to feel deliberate; maps to `ZoomConfig::idle_release_ms`.
- `smoothness: f32` - follow damping applied to pan and zoom motion (0 = snappy, 1 = very sluggish). Default `0.10`. *Why:* small damping keeps the zoom responsive while removing jitter from fast mouse movement.
- `clicks: u32` - number of clicks required to trigger a zoom-in. Default `1`. Clamped to at least 1 in `to_zoom_config`. *Why clamped:* a 0 value would fire on every frame; the clamp is a silent safety net.
- `camera_shrink: bool` - whether the camera panel scales down while zoomed. Default `true`. *Why:* a large bubble partly obscures the zoomed content; shrinking it keeps the action visible.
- `camera_shrink_min: f32` - the smallest scale the camera panel can reach during zoom. Default `0.62`. *Why:* prevents the camera from disappearing completely; 0.62 is small enough to be unobtrusive while still showing the presenter.
- `smart_hold: bool` - enables typing-aware hold extension (keystrokes extend the zoom hold). Default `true`. *Why opt-out rather than opt-in:* most recordings involve typing after clicking; holding the zoom through keystrokes is almost always the right behavior.
- `smart_follow: bool` - enables predictive pan that anticipates cursor direction. Default `false`. *Why off by default:* the feature is experimental and can feel jarring on recordings with erratic mouse movement.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.zoom`) - persisted in `config.json`
- `src-tauri/src/edit/seed.rs` - calls `to_zoom_config()` to seed an `EditDoc` from a raw recording
- `src-tauri/src/export/pipeline/exporter.rs` - calls `to_zoom_config()` when building the export pipeline
- `src-tauri/src/export/render/fromedit.rs` - calls `to_zoom_config()` when exporting from an `EditDoc`

## ZoomSettings::to_zoom_config

```rust
pub fn to_zoom_config(&self) -> ZoomConfig
```

Builds a `ZoomConfig` from the user-visible subset of settings, leaving all other `ZoomConfig` fields at their tuned internal defaults.

### Inputs

- `self` - the `ZoomSettings` snapshot. *Why a method rather than a `From` impl:* the conversion is one-directional and is always called on a snapshot loaded from settings, so a method on `ZoomSettings` is clearer at the call site.

### Returns

`ZoomConfig` with: `target_scale` forwarded directly; `idle_release_ms = self.hold_ms`; `follow_damping = self.smoothness`; `clicks_to_trigger = self.clicks.max(1)`; all other fields from `ZoomConfig::default()` (including `zoom_in_ms = 350`, `merge_window_ms`, and easing values).

### Implementation

1. Construct `ZoomConfig` using struct update syntax: set the four user-facing fields explicitly, then `..ZoomConfig::default()` fills the rest.
2. Clamp `clicks_to_trigger` with `.max(1)` inline. *Why at conversion time rather than storage time:* the stored value is preserved as-is; the clamp is applied only when producing the algorithm config.

### Behaviors

- `defaults_match_tuned_zoom_and_round_trip` - verifies that default `ZoomSettings` produce `target_scale = 2.2`, `idle_release_ms = 2200`, `follow_damping = 0.10`, `clicks_to_trigger = 1`, and that `zoom_in_ms = 350` (the untouched `ZoomConfig` default) survives.

## ClickFxStyle

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClickFxStyle { None, Ripple, Pulse, Glow, Shockwave, Particles, Neon }
```

Visual style for click effects rendered on the output video.

- `None` - no effect drawn. *Why:* allows disabling effects without toggling `ClickFxSettings::enabled` (which also gates spotlight and captions).
- `Ripple` - expanding ring. Default. *Why:* subtle and universally legible.
- `Pulse` - filled disc that fades. *Why:* softer alternative to a ring.
- `Glow` - expanding glow disc. *Why:* works well for dark themes.
- `Shockwave` - fast-expanding thin ring (additive blend). *Why:* more dramatic look for action-heavy demos.
- `Particles` - burst of particles from the click point. *Why:* adds energy to instructional content.
- `Neon` - ring with additive blend at higher opacity. *Why:* high-contrast option for dark-background recordings.

Serialises as lowercase. Each variant maps to a float shader ID in `export::fx_uniforms::style_id`.

### Used by

- `src-tauri/src/export/fx/clickdraw.rs` - selects which draw path to run per frame
- `src-tauri/src/export/fx/fx_uniforms.rs` (`style_id`) - converts to a float uniform for the GPU shader
- `src-tauri/src/settings/model.rs` (`ClickFxSettings.style`) - stored in the per-recording settings block

## SpotlightMode

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpotlightMode { Classic, Blur, Halo, Breathing, Nebula, Vignette }
```

Spotlight visual effect applied around the cursor when spotlight is active.

- `Classic` - sharp circular spotlight with dimmed surroundings. Default. *Why:* the clearest way to draw attention to a region; works on any content.
- `Blur` - soft blur outside the spotlight radius. *Why:* focuses attention without darkening.
- `Halo` - bright ring at the spotlight edge. *Why:* alternative emphasis style.
- `Breathing` - pulsing spotlight size. *Why:* draws eye movement to the cursor.
- `Nebula` - nebula-wash tint outside the spotlight. *Why:* ties in with the TCursor brand aesthetic.
- `Vignette` - dark vignette (gradient dim to edges). *Why:* cinematic look for presenter content.

Serialises as lowercase. Maps to a float shader ID in `export::fx_uniforms::spot_mode_id`.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` (`spot_mode_id`) - converts to a float uniform for the spotlight shader
- `src-tauri/src/export/fx/fxdraw.rs` - used in test fixtures to exercise spotlight rendering paths

## VideoFxMode

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VideoFxMode { NebulaWash, CinematicDim, ScreenFocus, ColorPop }
```

Full-frame video effect activated by the `video_fx_hold` hotkey.

- `NebulaWash` - nebula color overlay. Default. *Why:* the TCursor brand effect; visually distinct from standard recording tools.
- `CinematicDim` - overall dimming with a cinematic color grade. *Why:* produces a filmic look for product walkthroughs.
- `ScreenFocus` - desaturates the background, keeping the foreground in color. *Why:* draws attention to active content.
- `ColorPop` - saturates and brightens the entire frame. *Why:* energising effect for high-pace demos.

Serialises as lowercase. Maps to a float shader ID in `export::fx_uniforms::video_mode_id`.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` (`video_mode_id`) - converts to a float uniform for the video FX shader

## ClickFxSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ClickFxSettings {
    pub enabled: bool,
    pub style: ClickFxStyle,
    pub color: [u8; 3],
    pub intensity: f32,
    pub captions: bool,
    pub spotlight: bool,
    pub spotlight_dim: f32,
    pub spotlight_radius: f32,
    pub spotlight_feather: f32,
    pub spotlight_mode: SpotlightMode,
    pub spotlight_tint: [u8; 3],
    pub video_fx_mode: VideoFxMode,
    pub spotlight_dim_camera: bool,
}
```

All settings for click effects, spotlight overlays, and video FX applied to the output video.

Fields:

- `enabled: bool` - master gate for click effects and spotlight. Default `true`. *Why:* a single toggle that disables all visual effects for minimalist recordings.
- `style: ClickFxStyle` - which click effect to draw. Default `Ripple`.
- `color: [u8; 3]` - RGB color of the click effect. Default `[255, 255, 255]` (white). *Why white:* legible on any background without adjusting per-recording.
- `intensity: f32` - effect strength / opacity multiplier (0..1). Default `0.8`. *Why not 1.0:* full opacity can be harsh on light backgrounds; 0.8 blends more naturally.
- `captions: bool` - whether hotkey action names are rendered as text overlays at the click site. Default `false`. *Why opt-in:* captions suit tutorial recordings but are distracting for demos.
- `spotlight: bool` - whether the spotlight overlay activates with the hotkey. Default `false`. *Why opt-in:* an always-on spotlight is disorienting; users enable it deliberately for focused walkthroughs.
- `spotlight_dim: f32` - dimming factor applied outside the spotlight (0 = no dim, 1 = black). Default `0.60`. *Why:* keeps context visible while clearly emphasising the spotlight area.
- `spotlight_radius: f32` - spotlight circle radius as a fraction of canvas height. Default `0.13`. *Why height fraction:* stays proportionate at different resolutions.
- `spotlight_feather: f32` - soft falloff width at the spotlight edge (fraction of canvas height). Default `0.10`. *Why:* a hard edge looks abrupt; feathering blends the transition naturally.
- `spotlight_mode: SpotlightMode` - which spotlight visual to apply. Default `Classic`.
- `spotlight_tint: [u8; 3]` - RGB tint for spotlight modes that support it. Default `[130, 90, 255]` (purple). *Why purple:* matches the TCursor brand nebula palette.
- `video_fx_mode: VideoFxMode` - full-frame fx mode activated by `video_fx_hold`. Default `NebulaWash`.
- `spotlight_dim_camera: bool` - whether the spotlight dim also darkens the webcam PiP. Default `true` (today's behavior: the camera dims like everything else outside the lit zone). *Why default true:* preserves byte-identical output for existing recordings/config files loaded before this field existed (`#[serde(default = "default_true")]` on the field, since the struct-level `#[serde(default)]` alone would fall back to `bool::default() == false`). Setting it `false` keeps the webcam fully lit while the spotlight still dims the rest of the frame - threaded through `Spot::dim_camera` in `fx_state.rs` to both the GPU shader (`fx.wgsl`'s `camcov` un-dim) and the CPU path (`spotdraw.rs`).

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.clickfx`) - persisted in `config.json`
- `src-tauri/src/export/fx/clickdraw.rs` - reads `style` and `color` to select and paint click effects per frame
- `src-tauri/src/export/fx/fx_uniforms.rs` - converts all fx settings to GPU shader uniforms
- `src-tauri/src/export/fx/fxdraw.rs` - constructs `FxState` from `ClickFxSettings` fields for rendering tests
- `src-tauri/src/export/fx/fx_state.rs` (`fx_state_at`) - reads `spotlight_dim_camera` to set `Spot::dim_camera`
- `src-tauri/src/export/fx/caption.rs` - reads `captions` to decide whether to render action text overlays

## HotkeySettings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct HotkeySettings {
    pub zoom_hold: String,
    pub spotlight_hold: String,
    pub video_fx_hold: String,
    pub layout_screen: String,
    pub layout_camera: String,
    pub layout_presenter: String,
    pub layout_screen_only: String,
    pub layout_camera_only: String,
}
```

Keyboard shortcut strings for runtime actions, stored as human-readable strings and parsed by the hotkey listener at startup.

Fields:

- `zoom_hold: String` - hold this key combo to manually activate auto-zoom. Default `"Ctrl+Alt+Z"`. *Why a hold, not a toggle:* zoom releases the moment the key is released, mirroring the click-triggered behavior.
- `spotlight_hold: String` - hold to activate the spotlight overlay. Default `"Ctrl+Alt+S"`.
- `video_fx_hold: String` - hold to activate full-frame video FX. Default `"Ctrl+Alt+V"`.
- `layout_screen: String` through `layout_camera_only: String` - switch the live layout to a named mode. Defaults `"Ctrl+Alt+1"` through `"Ctrl+Alt+5"`. *Why Ctrl+Alt:* avoids conflicts with common application shortcuts that use Ctrl or Alt alone.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.hotkeys`) - persisted in `config.json`
- `src-tauri/src/actions/matcher.rs` (`arming_from_settings`) - converts hotkey strings to `Arm` structs for the `ActionMatcher`
- `src-tauri/src/export/fx/caption.rs` (`caption_at`) - reads hotkey strings to build action label text for caption overlays

## ThemeMode

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode { Light, Dark, System }
```

UI color theme selection.

- `Light` - light theme regardless of OS setting. Default (via `InterfaceSettings`). *Why default:* most tutorial recordings are made on light-themed desktops; light default avoids an unexpected dark HUD on first launch.
- `Dark` - dark theme regardless of OS setting.
- `System` - defers to the OS dark-mode preference via `win::theme::os_prefers_dark`.

Serialises as lowercase.

### Used by

- `src-tauri/src/win/theme.rs` (`resolve_dark`) - maps `System` to an OS registry query; used to decide whether to invert cursor sprites and apply dark-theme coloring
- `src-tauri/src/export/pipeline/exporter.rs` - calls `resolve_dark(settings.ui.theme)` to determine sprite inversion during export

## InterfaceSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct InterfaceSettings {
    pub theme: ThemeMode,
    pub accent: [u8; 3],
}
```

UI theming configuration.

Fields:

- `theme: ThemeMode` - which color theme to apply to the HUD. Default `ThemeMode::Light`.
- `accent: [u8; 3]` - RGB accent color used for interactive elements throughout the UI. Default `[239, 68, 68]` (red). *Why red:* vivid, on-brand default that reads well against both light and dark backgrounds.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.ui`) - persisted in `config.json`
- `src-tauri/src/export/pipeline/exporter.rs` - reads `ui.theme` to resolve dark mode for cursor sprite inversion

## CursorStyle

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle { System, Enhanced, Hidden }
```

Controls how the cursor appears in the output video.

- `System` - the OS cursor is baked directly into the Windows Graphics Capture frame. Default. *Why default:* zero-latency, zero-overhead; the OS composites the cursor for free and the result is always pixel-perfect.
- `Enhanced` - the OS cursor is excluded from WGC capture; TCursor draws its own sprite with motion blur, bounce animations, and per-type icons. *Why excluded rather than replaced:* drawing on top of the captured cursor would cause a double cursor; exclusion ensures only the enhanced sprite appears.
- `Hidden` - the OS cursor is excluded and nothing is drawn in its place. *Why:* suitable for recordings where the presenter moves the mouse for internal navigation but does not want the cursor on screen.

Serialises as lowercase.

### Used by

- `src-tauri/src/settings/model.rs` (`CursorStyle::captures_os_cursor`) - queried to configure WGC
- `src-tauri/src/session/record/recorder.rs` - calls `captures_os_cursor()` to set the WGC cursor inclusion flag; checks `== Enhanced` to start the `CursorTypeTracker`
- `src-tauri/src/export/cursor/cursorset.rs` (`prep`) - returns `None` early for any style except `Enhanced`, skipping sprite preparation

## CursorStyle::captures_os_cursor

```rust
pub fn captures_os_cursor(self) -> bool
```

Returns `true` only for `CursorStyle::System`.

### Inputs

- `self: CursorStyle` - the configured cursor style. *Why a method on the enum:* eliminates repeated `matches!` at every call site; the logic stays adjacent to the type it describes.

### Returns

`true` for `System`; `false` for `Enhanced` and `Hidden`. When `false`, WGC should be configured to exclude the OS cursor from the capture stream so TCursor can draw its own (or nothing).

### Behaviors

- `partial_json_fills_defaults` in `model.rs` - asserts `System.captures_os_cursor() == true`, `Enhanced.captures_os_cursor() == false`, `Hidden.captures_os_cursor() == false`.

## CursorSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,
    pub motion_blur: f32,
    pub click_bounce: bool,
    pub bounce_intensity: f32,
}
```

Per-cursor appearance and animation settings. Applies only when `style == Enhanced`.

Fields:

- `style: CursorStyle` - rendering mode. Default `CursorStyle::System`.
- `size: f32` - scale factor applied to the base cursor sprite size (1.0 = default). Default `1.0`. *Why:* lets users with high-DPI recordings scale the cursor up so it remains readable on smaller playback screens.
- `motion_blur: f32` - trail strength for the motion blur effect (0 = off, 1 = maximum). Default `0.35`. *Why 0.35:* noticeable but not overwhelming on fast pans; zero would make the sprite look teleporting.
- `click_bounce: bool` - whether the cursor sprite plays a bounce-dip animation on mouse down. Default `true`. *Why on by default:* the bounce makes click detection trivially legible without any visual effect ring.
- `bounce_intensity: f32` - depth of the bounce dip on a 0..1 scale (0.5 gives approx 0.18 scale-dip, 1.0 gives approx 0.36 dip). Default `0.5`. *Why not 1.0:* a full dip at 1.0 looks cartoonish; 0.5 gives a subtle-but-readable response.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.cursor`) - persisted in `config.json`
- `src-tauri/src/export/cursor/cursorset.rs` (`prep`) - reads `style`, `motion_blur`, `click_bounce`, `bounce_intensity` to configure cursor sprite animation
- `src-tauri/src/session/record/recorder.rs` - reads `style.captures_os_cursor()` and checks `style == Enhanced` to initialize the cursor type tracker

## Settings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Settings {
    pub zoom: ZoomSettings,
    pub clickfx: ClickFxSettings,
    pub hotkeys: HotkeySettings,
    pub appearance: AppearanceSettings,
    pub cursor: CursorSettings,
    pub ui: InterfaceSettings,
    pub audio_offset_ms: i32,
}
```

Top-level settings struct persisted to and loaded from `config.json` via `settings::store`. Derives `Default` so `Settings::default()` always produces a fully usable configuration without reading any file.

Fields:

- `zoom: ZoomSettings` - auto-zoom algorithm settings.
- `clickfx: ClickFxSettings` - click effects, spotlight, and video FX settings.
- `hotkeys: HotkeySettings` - keyboard shortcut strings.
- `appearance: AppearanceSettings` - per-mode layout appearance fractions.
- `cursor: CursorSettings` - cursor rendering mode and animation parameters.
- `ui: InterfaceSettings` - HUD theme and accent color.
- `audio_offset_ms: i32` - manual mic-vs-video sync nudge in milliseconds. Negative values pull the mic track earlier to cancel device input latency. Zero means no adjustment. Default `0`. *Why signed:* input latency is subtractive; positive values also exist to handle rare setups where the mic arrives ahead of video.

### Used by

- `src-tauri/src/settings/store.rs` (`load`, `save`) - the struct that is serialised to and deserialised from disk
- `src-tauri/src/commands.rs` (`get_settings`, `set_settings`) - surfaced over IPC so the frontend can read and write settings
- `src-tauri/src/session/record/recorder.rs` - loaded at recording start via `store::load()` to snapshot all settings for the session
- `src-tauri/src/export/pipeline/exporter.rs` - received from the IPC call and drives every export subsystem
