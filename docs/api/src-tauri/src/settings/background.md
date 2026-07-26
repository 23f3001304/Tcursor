# src-tauri/src/settings/background.rs

User-facing background-style settings: which of the (small) set of real background types to render, plus their parameters. Split out of `model.rs` to keep that file under the 200-line budget. See `export::scene::background::build` for how a `BackgroundSettings` value becomes pixels.

## BackgroundKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind { Mesh, Solid, Gradient }
```

Which background variant `BackgroundSettings` currently describes.

- `Mesh` - the bundled default background image (`assets/backgrounds/bg.jpg`). Default. *Why default:* reproduces today's fixed background exactly, so every recording/config saved before this field existed keeps its exact look.
- `Solid` - a flat user-chosen color (`BackgroundSettings.solid`).
- `Gradient` - a two-stop linear gradient at a chosen angle (`BackgroundSettings.gradient_from`/`gradient_to`/`gradient_angle_deg`).

Custom image/video backgrounds are intentionally NOT modeled here yet - they need a real upload + asset-storage subsystem. `BackgroundPanel` flags them as "coming soon" in the UI rather than faking a variant with no real content behind it.

Serialises as lowercase.

### Used by

- `src-tauri/src/export/scene/background.rs` (`build`) - dispatches on `kind` to pick the render path (ffmpeg decode for `Mesh`, direct rasterization for `Solid`/`Gradient`).
- `src/editor/panels/BackgroundPanel.tsx` - the segmented Background Type control writes this (for the two real kinds; `image`/`video` tabs are display-only).

## BackgroundSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub kind: BackgroundKind,
    pub solid: [u8; 3],
    pub gradient_from: [u8; 3],
    pub gradient_to: [u8; 3],
    pub gradient_angle_deg: f32,
    pub blur: f32,
}
```

Part of `Settings` (persisted in both `config.json` and per-project `edit.json`).

Fields:

- `kind: BackgroundKind` - which render path to use. Default `Mesh`.
- `solid: [u8; 3]` - RGB color used when `kind == Solid`. Default `[24, 24, 30]` (dark neutral, close to the mesh's average tone).
- `gradient_from: [u8; 3]`, `gradient_to: [u8; 3]`, `gradient_angle_deg: f32` - the two-stop gradient used when `kind == Gradient`. Defaults `[36, 41, 56]` -> `[88, 64, 120]` at `135.0` degrees (matches `export::types::Background::default()`, the pre-existing gradient fallback). *Why only two stops:* mirrors `export::types::Background::Gradient` exactly - a 3+ stop gradient would need a new render-side variant, so the editor's gradient presets are curated to two-stop equivalents rather than faking a mismatch between the preset swatch and the real render.
- `blur: f32` - 0..1 softness applied once to the static background buffer. Default `0.0` (off, today's behavior). *Why safe to apply per-rebuild rather than per-frame:* `bg` is built once per export/preview (`FrameRenderer::new`/`reload_edit`), never per output frame, so even a non-trivial blur pass costs nothing in the steady state.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.background`) - persisted in `config.json` and `edit.json`.
- `src-tauri/src/export/scene/background.rs` (`build`) - the sole consumer that turns this into pixels.
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`, `FrameRenderer::reload_edit`) - `reload_edit` compares old vs. new `BackgroundSettings` (via `PartialEq`) to decide whether to rebuild `bg` at all.
- `src/editor/panels/BackgroundPanel.tsx` - reads/writes every field except the two backend-less UI tabs (image/video).
