# src-tauri/src/settings/background.rs

User-facing background-style settings: which of the (small) set of real background types to render, plus their parameters. Split out of `model.rs` to keep that file under the 200-line budget. See `export::scene::background::build` for how a `BackgroundSettings` value becomes pixels.

## BackgroundKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundKind { Mesh, Solid, Gradient, Image, Video }
```

Which background variant `BackgroundSettings` currently describes.

- `Mesh` - a bundled background IMAGE. Default. Which one is decided by `BackgroundSettings.mesh`: empty (the default, and what every pre-library doc loads as) means the legacy `assets/backgrounds/bg.jpg`, so every recording/config saved before the wallpaper library existed keeps its exact look; a non-empty id names one of `settings::wallpapers::WALLPAPERS`.
- `Solid` - a flat user-chosen color (`BackgroundSettings.solid`).
- `Gradient` - a two- or three-stop linear gradient at a chosen angle (`BackgroundSettings.gradient_from`/`gradient_mid`/`gradient_to`/`gradient_angle_deg`).
- `Image` - the user's own still image, imported into the project (`BackgroundSettings.asset`). Decoded cover-fit into the same static buffer the wallpapers use.
- `Video` - the user's own moving background, imported into the project. A GIF is a `Video`: one kind, so the export has one decode path for both. The export streams it frame by frame on the output clock (`export::pipeline::bg_pipe`); the still buffer holds its FIRST frame, which is what the Rust preview commands show and what the export falls back to if the stream dies.

An asset whose file is missing (project moved, file deleted outside the app) renders the base wallpaper/colour instead - never a failure, never a black frame. See `export::scene::background::build`.

Serialises as lowercase.

### Used by

- `src-tauri/src/export/scene/background.rs` (`build`) - dispatches on `kind` to pick the render path (ffmpeg decode for `Mesh`, direct rasterization for `Solid`/`Gradient`).
- `src/editor/panels/BackgroundPanel.tsx` - the segmented Background Type control (Wallpapers / Color / Gradient) writes this.

## BackgroundSettings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct BackgroundSettings {
    pub kind: BackgroundKind,
    pub solid: [u8; 3],
    pub gradient_from: [u8; 3],
    pub gradient_to: [u8; 3],
    pub gradient_angle_deg: f32,
    pub blur: f32,
    #[serde(default)] pub mesh: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub gradient_mid: Option<[u8; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub asset: Option<String>,
    #[serde(default)] pub dim: f32,
}
```

*Why no longer `Copy`:* `mesh` is an owned `String`. Nothing relied on the copy - the struct is only ever read behind a reference (`background::build`), compared (`FrameRenderer::reload_edit`), or moved as part of `Settings`, which was never `Copy` either.

Part of `Settings` (persisted in both `config.json` and per-project `edit.json`).

Fields:

- `kind: BackgroundKind` - which render path to use. Default `Mesh`.
- `solid: [u8; 3]` - RGB color used when `kind == Solid`. Default `[24, 24, 30]` (dark neutral, close to the mesh's average tone).
- `gradient_from: [u8; 3]`, `gradient_to: [u8; 3]`, `gradient_angle_deg: f32` - the gradient's end stops and direction, used when `kind == Gradient`. Defaults `[36, 41, 56]` -> `[88, 64, 120]` at `135.0` degrees (matches `export::types::Background::default()`, the pre-existing gradient fallback).
- `mesh: String` - which bundled wallpaper `Mesh` renders (`settings::wallpapers::WALLPAPERS` id). Default EMPTY, which means the legacy `bg.jpg`. *Why empty rather than an id like `"classic"`:* every project and config file written before this field existed deserializes to the field type's default, so "the value those docs load" and "the legacy background" have to be the same value - naming the legacy image in the table instead would have made the pre-library default depend on a table entry that could later be renamed.
- `gradient_mid: Option<[u8; 3]>` - optional middle stop for `Gradient`, sitting at the ramp's midpoint. Default `None`, and `skip_serializing_if` keeps it out of the JSON entirely when unset, so adding the field did not touch a single existing `edit.json`. Passed through to `export::types::Background::Gradient`'s own `mid`.
- `blur: f32` - 0..1 softness applied once to the static background buffer. Default `0.0` (off, today's behavior). *Why safe to apply per-rebuild rather than per-frame:* `bg` is built once per export/preview (`FrameRenderer::new`/`reload_edit`), never per output frame, so even a non-trivial blur pass costs nothing in the steady state. *Consequence for `Video`:* a one-off pass over the static buffer reaches that background's FIRST frame only (the fallback still), not the streamed frames - so `BackgroundPanel` hides the Blur slider while `kind == Video` rather than leaving a control that visibly does nothing. Rust is unchanged either way.
- `asset: Option<String>` - the imported background file, RELATIVE to the project folder (`background/<file>`), used by `Image`/`Video`. Default `None`, and `skip_serializing_if` keeps it out of every `edit.json` that has none. *Why relative:* a project folder is copyable/movable, and an absolute path would break the moment it was; `settings::bg_asset::asset_path` refuses anything absolute or containing `..` when resolving it back. *Why it survives switching away:* picking a wallpaper, colour or gradient only changes `kind` - the file stays on disk and in this field, so re-selecting the asset card restores it with no re-import. Only the card's own Remove deletes the file and clears this.
- `dim: f32` - 0..0.8 black overlay over whichever background actually has pixels (wallpaper, image, video). Default `0.0`. Read through `dim_clamped`. Applied exactly ONCE, by whichever side owns the pixels: Rust for the static buffer and each streamed video frame, `src/editor/stage/stageBg.ts` for the preview's own `<video>`/GIF draw (the preview's still path consumes an already-dimmed PNG from `preview_bg`, so re-applying there would square it).

## dim_clamped

```rust
pub fn dim_clamped(&self) -> f32
```

`dim` clamped into `[0.0, 0.8]`. The panel's slider is already 0..80%, so this only matters for a hand-edited `edit.json`; 0.8 is the floor on legibility (a fully black background is what `Solid` is for). Every consumer reads this, never the raw field.

### Behaviors

- `default_is_the_legacy_mesh_and_round_trips` - the default is `Mesh` with an EMPTY `mesh` and no `gradient_mid`, and survives a serialize/deserialize round trip unchanged.
- `a_pre_wallpaper_doc_loads_as_the_legacy_bundled_image` - the exact JSON a project saved before the wallpaper library (no `mesh`, no `gradient_mid`) deserializes equal to `default()`, so it renders byte-identically.
- `an_unset_middle_stop_is_not_serialized` - `gradient_mid: None` writes no key at all; `Some(..)` does.
- `an_asset_background_round_trips_and_pre_asset_docs_are_untouched` - a `Video` + `asset` + `dim` value survives a round trip, a doc written before M6 loads as `(None, 0.0)`, and an unset `asset` writes no key.
- `dim_is_clamped_to_the_published_range` - `dim_clamped` maps -1.0/0.0/0.5/9.0 to 0.0/0.0/0.5/0.8.
- `the_two_new_kinds_serialise_lowercase` - `Image`/`Video` are `"image"`/`"video"` on the wire, and `{"kind":"image","asset":...}` deserializes.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.background`) - persisted in `config.json` and `edit.json`.
- `src-tauri/src/export/scene/background.rs` (`build`) - the sole consumer that turns this into pixels.
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`, `FrameRenderer::reload_edit`) - `reload_edit` compares old vs. new `BackgroundSettings` (via `PartialEq`) to decide whether to rebuild `bg` at all.
- `src-tauri/src/settings/bg_asset.rs` - imports/removes the file `asset` names and resolves it back to an absolute path.
- `src-tauri/src/export/pipeline/bg_pipe.rs` - streams a `Video` asset's frames on the output clock.
- `src/editor/panels/BackgroundPanel.tsx` - reads/writes every field (`mesh` from the Wallpapers grid, the gradient fields from `GradientTab`, `asset` from the asset card, `dim` from the Dim slider).
- `src/editor/stage/stageBg.ts` - the preview's own reading of `kind`/`asset`/`dim`.
