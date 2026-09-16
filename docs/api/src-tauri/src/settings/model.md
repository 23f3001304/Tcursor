# src-tauri/src/settings/model.rs

Defines the complete user-facing `Settings` tree: one top-level struct and all nested configuration structs and enums that are persisted to `config.json`, except `CursorStyle`/`CursorSettings`, which live in the sibling `settings/cursor.rs` (see `cursor.md`) purely to keep this file under the repo's 200-line limit - `Settings.cursor` still embeds `CursorSettings` here. Every struct carries `#[serde(default)]` so that old config files gain new fields silently, enabling additive schema evolution with no migration code.

## CamZoomAction

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CamZoomAction { Shrink { to: f32 }, Hide, Stay }
```

What the webcam PiP does while a zoom is active. Resolved per-zoom (`Zoom.cam_action`), falling back to the global `ZoomSettings::resolved_cam_action`.

Variants:

- `Shrink { to: f32 }` - the panel scales toward `to` (a fraction of full size) as the zoom deepens. *Why a payload rather than reusing `camera_shrink_min`:* a per-zoom override needs its own floor, independent of the global setting.
- `Hide` - the panel fades out (alpha -> 0) on the same smoothstepped progress. *Why alpha rather than shrinking to zero:* a fade reads as intentional; a panel collapsing to a point reads as a glitch.
- `Stay` - the panel is untouched by the zoom.

### Used by

- `src-tauri/src/export/scene/mod.rs` (`apply_cam_zoom_action`) - the single place the action is turned into a `Panel`
- `src-tauri/src/edit/model.rs` (`Zoom.cam_action`) - the per-zoom override
- `src/editor/stage/camera/camZoomAction.ts` - the TS preview mirror

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
    pub cam_zoom_default: Option<CamZoomAction>,
    pub camera_smoothing_ms: u32,
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
- `cam_zoom_default: Option<CamZoomAction>` - global default webcam-on-zoom action. Default `None`. *Why `Option` rather than a plain `CamZoomAction` default:* `None` means "derive from the legacy `camera_shrink`/`camera_shrink_min` pair", so every config written before this field existed keeps rendering exactly as it did. See `resolved_cam_action`.
- `camera_smoothing_ms: u32` - opt-in critically-damped smoothing pass on the auto-zoom CAMERA path, forwarded verbatim to `ZoomConfig::smoothing_ms` (see `export/camera/smoothing.md`). Default `0` (off, bit-identical export - see `smoothing_off_is_bit_identical`). *Why a separate field from `smoothness`:* `smoothness` above is the camera's follow damping (`ZoomConfig::follow_damping`); this is an independent post-pass filter with its own lag/smoothing tradeoff (120ms removes ~72% of the worst velocity spike for ~33ms of lag; 250ms -> -86% for ~83ms) - and is also distinct from `CursorSettings::smoothness` (see `cursor.md`), which is the unrelated CURSOR low-pass.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.zoom`) - persisted in `config.json`
- `src-tauri/src/edit/seed.rs` - calls `to_zoom_config()` to seed an `EditDoc` from a raw recording
- `src-tauri/src/export/pipeline/exporter.rs` - calls `to_zoom_config()` when building the export pipeline
- `src-tauri/src/export/render/fromedit.rs` - calls `to_zoom_config()` when exporting from an `EditDoc`

## ZoomSettings::resolved_cam_action

```rust
pub fn resolved_cam_action(&self) -> CamZoomAction
```

The global default webcam-on-zoom action, with back-compat folded in.

### Inputs

- `self` - the `ZoomSettings` snapshot. *Why a method:* the fallback depends on two other fields on the same struct, so resolution belongs next to them rather than at each call site.

### Returns

`cam_zoom_default` when set; otherwise `Shrink { to: camera_shrink_min }` if `camera_shrink` is on, else `Stay`.

### Behaviors worth knowing

- `resolved_default_derives_from_the_legacy_shrink_fields` (unit test): default settings resolve to `Shrink { to: 0.62 }`; `camera_shrink: false` resolves to `Stay`; an explicit `cam_zoom_default` beats both.
- `settings_json_without_cam_zoom_default_loads_and_resolves_to_shrink` (unit test): old JSON with no `cam_zoom_default` key deserializes to `None` and still resolves to today's shrink.

## ZoomSettings::to_zoom_config

```rust
pub fn to_zoom_config(&self) -> ZoomConfig
```

Builds a `ZoomConfig` from the user-visible subset of settings, leaving all other `ZoomConfig` fields at their tuned internal defaults.

### Inputs

- `self` - the `ZoomSettings` snapshot. *Why a method rather than a `From` impl:* the conversion is one-directional and is always called on a snapshot loaded from settings, so a method on `ZoomSettings` is clearer at the call site.

### Returns

`ZoomConfig` with: `target_scale` forwarded directly; `idle_release_ms = self.hold_ms`; `follow_damping = self.smoothness`; `clicks_to_trigger = self.clicks.max(1)`; `smoothing_ms = self.camera_smoothing_ms`; all other fields from `ZoomConfig::default()` (including `zoom_in_ms = 350`, `merge_window_ms`, and easing values).

### Implementation

1. Construct `ZoomConfig` using struct update syntax: set the five user-facing fields explicitly, then `..ZoomConfig::default()` fills the rest.
2. Clamp `clicks_to_trigger` with `.max(1)` inline. *Why at conversion time rather than storage time:* the stored value is preserved as-is; the clamp is applied only when producing the algorithm config.

### Behaviors

- `defaults_match_tuned_zoom_and_round_trip` - verifies that default `ZoomSettings` produce `target_scale = 2.2`, `idle_release_ms = 2200`, `follow_damping = 0.10`, `clicks_to_trigger = 1`, and that `zoom_in_ms = 350` (the untouched `ZoomConfig` default) survives.
- `camera_smoothing_ms_round_trips_into_zoom_config` - a `ZoomSettings` JSON missing `camera_smoothing_ms` deserializes to `0` and `to_zoom_config().smoothing_ms == 0`; an explicit `camera_smoothing_ms: 120` carries through to `smoothing_ms` unchanged.

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

- `src-tauri/src/export/fx/click/clickdraw.rs` - selects which draw path to run per frame
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
- `src-tauri/src/export/fx/click/clickdraw.rs` - reads `style` and `color` to select and paint click effects per frame
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

## LayoutPreset

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayoutPreset {
    pub id: String,
    pub name: String,
    pub appearance: AppearanceSettings,
}
```

One saved "look" (2026-09-14): a name plus a snapshot of **all five** layouts' appearance, for the editor's Layouts panel.

- `id: String` - opaque and stable; the frontend mints `lp1`, `lp2`, ... A rename changes `name` only, so a row keeps its identity.
- `name: String` - what the user typed, already trimmed and validated frontend-side (non-empty, unique case-insensitively including against the built-in "Default" row, at most 40 characters). Rust does not re-validate: the list is whatever the frontend last wrote, exactly like the rest of `Settings`.
- `appearance: AppearanceSettings` - all five layouts at once. A look is deliberately not per-layout: a coherent look is the relationship between the layouts a recording cuts among, so applying one is a single write of a project's `settings.appearance`.

**No `#[serde(default)]` on the struct itself**, and no `Default` impl: a preset is never partially present. It either exists in the list or it does not, and the list's own `#[serde(default)]` (on `Settings::layout_presets`) is what handles a config written before presets existed.

The backend only stores and returns these - nothing in the render path reads them. The export resolves a project's own `settings.appearance` through `AppearanceSettings::for_id` / `layout_for` / `overlay_for` (`settings::appearance`), which is unchanged, so presets needed no renderer work at all.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings::layout_presets`) - persisted in `config.json`
- `src/hud/settings/settings.ts` (`LayoutPreset`) - the TypeScript mirror
- `src/editor/panels/layout/LayoutsPanel.tsx` / `layoutPresets.ts` - the only reader and writer

## Settings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub zoom: ZoomSettings,
    pub clickfx: ClickFxSettings,
    pub hotkeys: HotkeySettings,
    pub appearance: AppearanceSettings,
    pub cursor: CursorSettings,
    pub ui: InterfaceSettings,
    pub audio_offset_ms: i32,
    pub background: BackgroundSettings,
    pub audio_mic_volume: f32,
    pub audio_sys_volume: f32,
    pub ai_model: String,
    #[serde(default)] pub layout_presets: Vec<LayoutPreset>,
    pub captions: CaptionStyle,
    #[serde(default)] pub motion: MotionSettings,
    #[serde(default)] pub grade: GradeSettings,
}
```

Top-level settings struct persisted to and loaded from `config.json` via `settings::store`. Has a MANUAL `impl Default` (not derived): `audio_mic_volume`/`audio_sys_volume` need `1.0` (unity gain), which a derived `Default` would get wrong (`f32::default() == 0.0`, silently muted audio) since those two fields have no nested type of their own to carry a custom `Default` impl the way every other field does.

Fields:

- `zoom: ZoomSettings` - auto-zoom algorithm settings.
- `clickfx: ClickFxSettings` - click effects, spotlight, and video FX settings.
- `hotkeys: HotkeySettings` - keyboard shortcut strings.
- `appearance: AppearanceSettings` - per-mode layout appearance fractions.
- `cursor: CursorSettings` - cursor rendering mode and animation parameters.
- `ui: InterfaceSettings` - HUD theme and accent color.
- `audio_offset_ms: i32` - manual mic-vs-video sync nudge in milliseconds. Negative values pull the mic track earlier to cancel device input latency. Zero means no adjustment. Default `0`. *Why signed:* input latency is subtractive; positive values also exist to handle rare setups where the mic arrives ahead of video.
- `background: BackgroundSettings` - background style (mesh/solid/gradient + blur). Default `BackgroundSettings::default()` (`Mesh`, today's bundled image, byte-identical to before this field existed). See `settings::background`.
- `audio_mic_volume: f32`, `audio_sys_volume: f32` - linear gain multipliers applied to each track at mux (0 = muted, 1 = unchanged, up to 1.5). Default `1.0` for both. *Why an explicit field-level `#[serde(default = "default_volume")]` in addition to the manual `impl Default` above:* belt-and-suspenders matching `spotlight_dim_camera`'s pattern, so a config saved without this key loads full volume under either code path.
- `ai_model: String` - Ollama model name for the AI director. Default `""` (empty = let the backend pick its own default, `"llama3.2"`), so configs saved before this field existed behave identically.
- `motion: MotionSettings` - the project's ONE motion language (M3): the curve pair every newly added zoom, layout segment and camera keyframe inherits, and what `EditOp::ApplyMotionDefault` stamps onto the ones already placed. Lives in the sibling `settings/motion.rs` (see `motion.md`) rather than here, both for this file's line budget and because the default has a real story behind it. Default Soft, which is the bare word `"smooth"` - exactly what the add ops used to hardcode - so a config written before this field existed behaves identically and every existing region reads back as Soft rather than Custom.
- `layout_presets: Vec<LayoutPreset>` - the user's saved layout looks, newest last. Default empty. *Why an explicit field-level `#[serde(default)]` on top of the container's:* same belt-and-suspenders as the two volumes - a `config.json` written before this field existed must load with an empty list under either code path rather than failing the whole `Settings` parse and silently resetting every other setting. `layout_presets_round_trip_and_default_empty` (`model_tests.rs`) pins both halves: `{}` loads empty, and a saved look survives a write/read cycle with all five layouts intact.
- `captions: CaptionStyle` - the caption look plus its ASR inputs (`settings::captions::CaptionStyle`). Distinct from `clickfx.captions`, the OLD hotkey-chord toggle, which keeps its name and meaning unchanged.
- `grade: GradeSettings` - the colour grade (spec 3.1, `docs/superpowers/specs/2026-09-15-editor-parity-features-design.md`): a preset name plus the three absolute numbers it seeds (`exposure`, `contrast`, `vignette`); the preset's other eight parameters stay a fixed lookup Batch 2b resolves in `export/grade`. Lives in the sibling `settings/grade.rs` (see `grade.md`), both for this file's line budget and because the model has a real story behind it, the same reason as `motion`. Default the identity, so a config written before this field existed loads and exports byte-identically.

### Used by

- `src-tauri/src/settings/store.rs` (`load`, `save`) - the struct that is serialised to and deserialised from disk
- `src-tauri/src/commands.rs` (`get_settings`, `set_settings`) - surfaced over IPC so the frontend can read and write settings
- `src-tauri/src/session/record/recorder.rs` - loaded at recording start via `store::load()` to snapshot all settings for the session
- `src-tauri/src/export/pipeline/exporter.rs` - received from the IPC call and drives every export subsystem
- `src-tauri/src/ai/commands.rs` (`ai_propose`) - receives `ai_model` (via the frontend passing `doc.settings.ai_model || undefined`)
- `src-tauri/src/export/pipeline/audio_mux.rs` (`mux`) - receives `audio_mic_volume`/`audio_sys_volume` via `RenderMeta`
