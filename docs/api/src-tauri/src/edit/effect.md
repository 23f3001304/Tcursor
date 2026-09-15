# src-tauri/src/edit/effect.rs

The effect region (`EffectKind`, `EffectRegion`), moved out of `edit/model.rs` verbatim for headroom (2026-09-15). `edit::model` re-exports both, so every existing path still resolves; `EditDoc.effects` is the list that carries them. The fade default (`default_fade_ms`, 250) stays in `edit/model.rs` next to the zoom defaults it was written with.

## EffectKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight }
```

The kind of an editable effect region. Serializes lowercase (`"spotlight"`) to match the TS `EffectKind`. The set grows over phases (video FX, captions later).

## EffectRegion

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EffectRegion {
    pub id: String, pub kind: EffectKind, pub start_ms: u32, pub end_ms: u32,
    pub fade_in_ms: u32, pub fade_out_ms: u32,
    pub mode: Option<crate::settings::model::SpotlightMode>,
    pub dim: Option<f32>,
    pub radius: Option<f32>,
    pub feather: Option<f32>,
    pub layer: u32,
}
```

An editable effect region on the timeline (v1: Spotlight). `EditDoc.effects` is a `Vec<EffectRegion>` with `#[serde(default)]` for back-compat (a pre-existing `edit.json` without `effects` loads). Params default from settings for now; at export, `fx_state::spotlight_region_alpha` fades a Spotlight region in/out over its span and unions it with the settings + hotkey-hold spotlight.

- `layer` - *priority when this region overlaps another Spotlight region; higher wins.*
