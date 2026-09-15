# src-tauri/src/settings/captions.rs

The caption look (`CaptionPos`, `CaptionSize`, `CaptionStyle`), split out on its own for M5 the way `InterfaceSettings` already was. `Settings.captions` holds one `CaptionStyle`. Distinct from `Settings.clickfx.captions`, the OLD hotkey-chord toggle (`export/fx/click/hotkeycap.rs`), which keeps its name and meaning unchanged forever.

## CaptionPos

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionPos { #[default] Bottom, Top }
```

Where the caption band sits on the output frame. Serializes lowercase (`"bottom"` / `"top"`) to match the TS `CaptionPos`.

- `Bottom` - the conventional caption position. Default.
- `Top` - clears the bottom of the frame for content that lives there (e.g. a taskbar demo).

### Used by

- `src-tauri/src/settings/captions.rs` - `CaptionStyle.position`
- `src/hud/settings/settings.ts` - `CaptionPos` TS mirror

## CaptionSize

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionSize { S, #[default] M, L }
```

Caption font-size rung. Serializes lowercase (`"s"` / `"m"` / `"l"`) to match the TS `CaptionSize`.

### Used by

- `src-tauri/src/settings/captions.rs` - `CaptionStyle.size`
- `src/hud/settings/settings.ts` - `CaptionSize` TS mirror

## CaptionSize::height_frac

```rust
pub fn height_frac(self) -> f32
```

The size rung as a fraction of output frame height, the same vocabulary every other height-based layout constant in this codebase uses (e.g. `ModeAppearance.cam_size`).

### Returns

`0.030` for `S`, `0.038` for `M`, `0.048` for `L`.

### Behaviors

- `size_rungs_are_fractions_of_output_height` - pins all three values.

## CaptionStyle

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CaptionStyle {
    pub enabled: bool,
    pub position: CaptionPos,
    pub size: CaptionSize,
    pub pill: bool,
    pub highlight: bool,
    pub model: String,
    pub language: String,
}
```

The caption look plus the two ASR inputs the Captions panel edits alongside it (ADDED-4: both belong to the project, so one struct carries them). Rendered in Rust for export and mirrored in TS for the preview, like every other overlay.

- `enabled` - *master on/off for the caption overlay. Default `true`.*
- `position: CaptionPos` - *bottom or top band. Default `Bottom`.*
- `size: CaptionSize` - *font-size rung. Default `M`.*
- `pill` - *whether a background pill is drawn behind the text for legibility. Default `true`.*
- `highlight` - *whether the active word is highlighted as it is spoken (needs `Caption.words`). Default `true`.*
- `model` - *the whisper model id to transcribe with (e.g. `"base.en"`, `"small.en"`). Default `"base.en"`.*
- `language` - *ASR language hint (`"en"` or `"auto"` with a multilingual model). Default `"en"`.*

**Not here:** the accent color. The renderer reads `InterfaceSettings::accent` (`settings::interface`), so there is exactly one accent in the doc rather than a second copy that could drift from it.

### Used by

- `src-tauri/src/settings/model.rs` - `Settings.captions`
- `src/hud/settings/settings.ts` - `CaptionStyle` TS mirror

### Behaviors

- `defaults_are_bottom_medium_pill_and_base_en` - pins every default field.
- `a_settings_blob_written_before_captions_existed_loads_with_the_default_style` - `{}` deserializes `Settings.captions` to `CaptionStyle::default()`, and confirms `Settings.clickfx.captions` (the OLD hotkey toggle) is untouched.

## CaptionAnim

```rust
pub enum CaptionAnim { None, Fade, Rise, Pop, Words }
```

How a caption enters and leaves (owner request 2026-09-15: "the way they come"). `fade` is the default and what every earlier project had (the 120 ms alpha ramp); `none` cuts; `rise` fades while drifting up half a line into place; `pop` fades while scaling from 92% to 100%; `words` reveals the caption word by word at each word's own start time, falling back to `fade` for a caption with no word timings. Serialised lowercase.

## CaptionStyle::height_frac

```rust
pub fn height_frac(&self) -> f32
```

The line height as a fraction of the frame height: `font_pct / 100` clamped to 1.5% to 8% when the fine size is set, else the `size` rung's value. New fields on `CaptionStyle` (all `#[serde(default)]`, so every existing `edit.json` loads unchanged): `font_pct: f32` (0 = use the rung), `text_color: [u8; 3]` (white), `highlight_color: Option<[u8; 3]>` (`None` = the interface accent, the old behaviour), `pill_color: [u8; 3]` (black), `pill_alpha: u8` (percent, 62 = the old `SCRIM`), `animation: CaptionAnim` (`fade`), `animation_ms: u32` (120 = the old `FADE_MS`). The defaults reproduce the previous look exactly, which is what makes the fields safe to add under a project that never touched them.
