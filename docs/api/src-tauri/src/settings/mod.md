# src-tauri/src/settings/mod.rs

MODULE OVERVIEW: The `settings` module defines the complete user-configurable state of TCursor and handles its persistence to `config.json`. The three submodules form a clean stack: `model` owns all types and enums (no I/O), `appearance` owns the layout/webcam geometry types and conversion helpers that translate fractional settings to pixel values, and `store` handles the single file that persists everything. Every struct and field carries `#[serde(default)]` so older config files gain new fields silently with no migration code. Callers in `recorder.rs` snapshot settings at recording start, while export and editor commands receive them over Tauri IPC.

## appearance

Per-mode layout and webcam geometry settings, stored as canvas fractions so they are resolution-independent. Also provides the two conversion helpers that map fractional settings to pixel-valued render types. Key items: `AppearanceSettings` (five `ModeAppearance` blocks, one per layout mode), `ModeAppearance` (fractional fields for padding, screen scale/radius, camera size/shape/corner/margins), `CamShape` (circle/rounded/rect), `CamCorner` (four anchor positions), `layout_for` (converts `ModeAppearance` to pixel-valued `Layout`), `overlay_for` (converts `ModeAppearance` to pixel-valued `OverlayLayout`), `AppearanceSettings::for_id` (looks up the block for a given `LayoutId`).

## model

All user-facing `Settings` types: enums, nested config structs, and the top-level `Settings` struct. Key items: `Settings` (top-level struct with fields `zoom`, `clickfx`, `hotkeys`, `appearance`, `cursor`, `ui`, `audio_offset_ms`), `ZoomSettings` (auto-zoom knobs) with `ZoomSettings::to_zoom_config` (builds internal `ZoomConfig`), `ClickFxSettings` (click effects, spotlight, video FX), `CursorSettings` with `CursorStyle` enum and `CursorStyle::captures_os_cursor`, `HotkeySettings` (key combo strings), `InterfaceSettings` with `ThemeMode` enum, `ClickFxStyle`, `SpotlightMode`, `VideoFxMode`.

## store

Thin persistence layer: computes the canonical config path, loads settings with silent fallback to defaults, and saves as pretty-printed JSON. Key items: `config_path` (returns `<platform-config-dir>/TCursor/config.json`), `load` (reads and deserializes, returns `Settings::default()` on any error), `save` (serializes to pretty JSON, creates parent dirs as needed).
