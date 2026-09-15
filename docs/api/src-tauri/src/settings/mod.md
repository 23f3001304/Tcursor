# src-tauri/src/settings/mod.rs

MODULE OVERVIEW: The `settings` module defines the complete user-configurable state of TCursor and handles its persistence to `config.json`. The submodules form a clean stack: `model` owns most types and enums (no I/O), `appearance` owns the layout/webcam geometry types and conversion helpers that translate fractional settings to pixel values, `background` owns the background-style settings type, and `store` handles the single file that persists everything. Every struct and field carries `#[serde(default)]` so older config files gain new fields silently with no migration code. Callers in `recorder.rs` snapshot settings at recording start, while export and editor commands receive them over Tauri IPC.

## appearance

Per-mode layout and webcam geometry settings, stored as canvas fractions so they are resolution-independent. Also provides the two conversion helpers that map fractional settings to pixel-valued render types. Key items: `AppearanceSettings` (five `ModeAppearance` blocks, one per layout mode), `ModeAppearance` (fractional fields for padding, screen scale/radius, camera size/shape/corner/margins), `CamShape` (circle/rounded/rect), `CamCorner` (four anchor positions), `layout_for` (converts `ModeAppearance` to pixel-valued `Layout`), `overlay_for` (converts `ModeAppearance` to pixel-valued `OverlayLayout`), `AppearanceSettings::for_id` (looks up the block for a given `LayoutId`).

## background

User-facing background-style settings, separated from `model.rs` to keep that file under the line budget. Key items: `BackgroundKind` (`Mesh`/`Solid`/`Gradient`/`Image`/`Video` - the bundled wallpapers, the two real color modes, and the user's own imported file), `BackgroundSettings` (kind + solid/gradient RGB fields + `blur` + `asset` + `dim`), `BackgroundSettings::dim_clamped`. See `export::scene::background::build` for how these become pixels.

## bg_asset

The user's own background file: copy it into `<project>/background/`, describe it, remove it - so `BackgroundSettings.asset` can stay a RELATIVE path and the project stays portable. Key items: `BackgroundAssetInfo`, `asset_kind_for` (extension -> `"image"`/`"video"`), `unique_name` (dedupe rather than overwrite), `asset_path` (resolve a stored relative path, refusing absolute paths and `..`), `thumb_rel`, `probe_asset` / `write_thumb` (both via the bundled ffmpeg/ffprobe - there is no image crate in this build), and the three commands `import_background_asset` / `background_asset_info` / `remove_background_asset`.

## captions

SCAFFOLD, OWNED BY TASK T1 of the M5 captions milestone. `asr::models::resolve_model` takes a `&CaptionStyle`, so the type has to exist for the `asr` module to compile; this file is the plan's binding data model verbatim and nothing more. Key items: `CaptionPos` (bottom/top), `CaptionSize` (s/m/l), `CaptionStyle` (the four look fields plus the two ASR preferences, `model` and `language`, which live here because the Captions panel edits them as one group and both belong to the project). T1 replaces it with the fuller version that carries `height_frac`, the `Settings.captions` field and its own tests.

## model

All user-facing `Settings` types: enums, nested config structs, and the top-level `Settings` struct. Key items: `Settings` (top-level struct with fields `zoom`, `clickfx`, `hotkeys`, `appearance`, `cursor`, `ui`, `audio_offset_ms`, `background`, `audio_mic_volume`, `audio_sys_volume`, `ai_model`), `ZoomSettings` (auto-zoom knobs) with `ZoomSettings::to_zoom_config` (builds internal `ZoomConfig`), `ClickFxSettings` (click effects, spotlight, video FX), `CursorSettings` with `CursorStyle` enum and `CursorStyle::captures_os_cursor`, `HotkeySettings` (key combo strings), `InterfaceSettings` with `ThemeMode` enum, `ClickFxStyle`, `SpotlightMode`, `VideoFxMode`.

## motion

The project's ONE motion language (M3), split off from `model.rs` for its line budget and for the story behind its default. Key item: `MotionSettings` (`preset` - the UI-facing name, provenance only; `easing` - the ramp INTO a region; `easing_out` - the ramp OUT of one), `#[serde(default)]` = Soft, and Soft is the bare word `"smooth"` so every region in every existing project reads back as Soft and the shipped camera trajectory stays bit-identical. Read by `edit::ops::motion` (the add ops and `ApplyMotionDefault`), written only by the editor's Settings > Motion; presets themselves live on the TypeScript side (`src/editor/motion/presets.ts`), since the editor is the only thing that ever authors a curve.

## theme

The one mapping from a configured `ThemeMode` to a concrete dark/light boolean, taking the desktop's own preference as an argument rather than reading it. Key item: `resolve_dark`. It lives here, not in `platform/`, because only the OS READ is platform code; the mapping is the same on every platform and `platform/mod.rs` holds nothing but `Platform` and `current()` since Batch D.

## store

Thin persistence layer: computes the canonical config path, loads settings with silent fallback to defaults, and saves as pretty-printed JSON. Key items: `config_path` (returns `<platform-config-dir>/TCursor/config.json`), `load` (reads and deserializes, returns `Settings::default()` on any error), `save` (serializes to pretty JSON, creates parent dirs as needed).
