# src-tauri/src/settings/interface.rs

The editor and HUD interface settings (`ThemeMode`, `InterfaceSettings`), moved out of `settings/model.rs` verbatim for headroom (2026-09-15). `settings::model` re-exports both, so every existing path still resolves; `Settings.ui` is the field that carries them.

## ThemeMode

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode { Light, Dark, System }
```

UI color theme selection.

- `Light` - light theme regardless of OS setting. Default (via `InterfaceSettings`). *Why default:* most tutorial recordings are made on light-themed desktops; light default avoids an unexpected dark HUD on first launch.
- `Dark` - dark theme regardless of OS setting.
- `System` - defers to the OS dark-mode preference via `platform::os_prefers_dark`.

Serialises as lowercase.

### Used by

- `src-tauri/src/platform/mod.rs` (`resolve_dark`) - maps `System` to the platform adapter's OS read (a registry query on Windows); used to decide whether to invert cursor sprites and apply dark-theme coloring
- `src-tauri/src/export/pipeline/exporter.rs` - calls `resolve_dark(settings.ui.theme)` to determine sprite inversion during export

## InterfaceSettings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct InterfaceSettings {
    pub theme: ThemeMode,
    pub accent: [u8; 3],
    pub animated_brand: bool,
    #[serde(default = "default_true")]
    pub interface_effects: bool,
    #[serde(default)]
    pub ai_choreography: bool,
}
```

UI theming configuration.

Fields:

- `theme: ThemeMode` - which color theme to apply to the HUD. Default `ThemeMode::Light`.
- `accent: [u8; 3]` - RGB accent color used for interactive elements throughout the UI. Default `[239, 68, 68]` (red). *Why red:* vivid, on-brand default that reads well against both light and dark backgrounds.
- `animated_brand: bool` (Task 39) - the "living brand" feel knob: whether `TcursorMark` (`src/shared/brand/TcursorMark.tsx`) flows/pulses for its recording/exporting/directing states at all, in the HUD titlebar and the editor's `TopBar`. Default `true`. *Why a settings field rather than always-on:* the fake-polish rule is every feel knob is a setting a user can turn off; `prefers-reduced-motion` disables the animation independently of this flag (accessibility isn't optional), but a user without that OS preference can still opt out here. Doesn't affect the dynamic Windows icon/taskbar progress (`shell::brand_icon`) - that's OS chrome, not an in-page animation, and stays purely state-driven.

- `interface_effects: bool` (micro-interaction pass, 2026-09-14) - the second feel knob, for the editor's **own** interface: the click ripple that blooms under every pointerdown in the chrome, and the magnetic pull the transport's Play button and Trim pills exert on a nearby pointer. Default `true`. Off unmounts the ripple overlay entirely (zero listeners, not a listener that early-returns) and turns the magnetic hook into a no-op - see `src/editor/effects/`. Like `animated_brand`, `prefers-reduced-motion` softens these regardless of the flag (the ripple stops growing, the pull stops entirely) and this flag is the opt-out for a user with no OS-level preference. It never touches the **export**: these are TCursor's own chrome, not the recording's click effects, which are `ClickFxSettings`.

  *Why the explicit `#[serde(default = "default_true")]`* when the container already carries `#[serde(default)]`: belt-and-suspenders, the same pattern `Settings::audio_mic_volume` uses. It is what makes a `config.json` written before this field existed load `true` rather than `bool::default()` - which would silently ship the feature turned off to every existing install. `model_tests.rs::interface_effects_defaults_on_and_round_trips` pins all three halves (the default is on, field-less JSON loads on, an explicit `false` survives a full `Settings` round-trip).

- `ai_choreography: bool` (M4 T5) - the AI Director's pointer replay: after the review sheet's Apply, the editor's fake pointer walks the applied edits, aiming and pressing at each one's pill (`src/editor/director/useAiRun.ts`). Default `false`. A flourish, and honest about it: the edits are already applied by the time it runs, this only performs them, which is why it is off until asked for. A plain `#[serde(default)]` is right here, since `false` IS the value a config written before the field existed should load with. `model_tests.rs::the_pointer_replay_is_off_until_the_user_asks_for_it` and `a_config_written_before_this_setting_existed_loads_with_it_off` pin both. Read doc-scoped (`doc.settings.ui.ai_choreography`), like `animated_brand`, and edited in the editor's Interface section, never the HUD's.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.ui`) - persisted in `config.json`
- `src/editor/director/useAiRun.ts` - reads `doc.settings.ui.ai_choreography` after an apply to decide whether the pointer replay runs
- `src-tauri/src/export/pipeline/exporter.rs` - reads `ui.theme` to resolve dark mode for cursor sprite inversion
- `src/editor/Editor.tsx` - reads `doc.settings.ui.animated_brand` (the per-recording snapshot) to gate `TopBar`'s `brandState`
- `src/hud/Hud.tsx` - reads the global `ui.animated_brand` (via `getSettings`/`Preferences`) to gate the titlebar mark's `state`
- `src/editor/effects/InterfaceEffects.tsx` - reads the global `ui.interface_effects` (via `getSettings`, on mount and on every window focus) and publishes it to the rest of the effects folder

