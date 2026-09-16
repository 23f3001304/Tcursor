# src-tauri/src/edit/effect.rs

The effect region (`EffectKind`, `EffectRegion`), moved out of `edit/model.rs` verbatim for headroom (2026-09-15). `edit::model` re-exports both, so every existing path still resolves; `EditDoc.effects` is the list that carries them. The fade default (`default_fade_ms`, 250) stays in `edit/model.rs` next to the zoom defaults it was written with.

## EffectKind

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EffectKind { Spotlight, Blur, Pixelate, Highlight }
```

The kind of an editable effect region. Serializes lowercase (`"spotlight"`/`"blur"`/`"pixelate"`/`"highlight"`) to match the TS `EffectKind`. `Blur`, `Pixelate` and `Highlight` are the three MASK kinds (added 2026-09-15, editor-parity batch 1) - a mask kind is any kind but `Spotlight`; see `EffectKind::is_mask`.

## EffectKind::is_mask

```rust
pub fn is_mask(self) -> bool
```

Whether `self` is a mask kind (`Blur`, `Pixelate` or `Highlight`) rather than `Spotlight`: `!matches!(self, EffectKind::Spotlight)`. Not called by the spotlight simulators - `spotlight_sim.rs`'s `SpotlightSim::winner` and `spotlightPreview.ts`'s `winner` match on `Spotlight` directly, so a mask region already cannot drive the spotlight without going through this method - `is_mask` is the one place the split is spelled out for code that instead needs to single masks out (the FX-lane pill, mask rendering and its inspector).

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
    pub rect: Option<[f32; 4]>,
    pub strength: Option<f32>,
    pub roundness: Option<f32>,
}
```

An editable effect region on the timeline: Spotlight, plus the three mask kinds. `EditDoc.effects` is a `Vec<EffectRegion>` with `#[serde(default)]` for back-compat (a pre-existing `edit.json` without `effects` loads). Params default from settings for now; at export, `fx_state::spotlight_region_alpha` fades a Spotlight region in/out over its span and unions it with the settings + hotkey-hold spotlight.

- `layer` - *priority when this region overlaps another Spotlight region; higher wins.*
- `rect` - *mask kinds only: `[x, y, w, h]` in CANVAS-FRACTION space - stored as fractions of the recorded canvas, `[0, 1]` on each axis, `(0,0)` at its top left (spec 1.1). Absent on every doc written before masks existed, and on every Spotlight region forever; a mask with no rect renders nothing rather than covering the frame.*
- `strength` - *blur radius or pixel-cell size, as a fraction of output height. Absent = the kind's default (`DEFAULT_BLUR` 0.020, `DEFAULT_PIXEL` 0.018; unused by Highlight).*
- `roundness` - *corner rounding of the mask rect, as a fraction of its SHORT side, 0..0.5. Absent = 0.06 (`DEFAULT_MASK_ROUNDNESS`).*
- `feather` - *Spotlight: its own soft edge. Masks: reused for the same purpose - the mask's soft edge width, a fraction of output height. Absent = 0.010 (`DEFAULT_MASK_FEATHER`).*
- `dim` - *Spotlight: max darkness outside the beam. Highlight (the mask kind): reused for the same purpose - how dark everything outside the rect goes. Absent = the global `clickfx.spotlight_dim` either way.*

### Behaviors

- `a_region_written_before_masks_existed_loads_with_no_rect` - a pre-mask `edit.json`'s Spotlight region deserializes with `rect`/`strength`/`roundness` all `None`, fades still defaulting to 250/250.
- `the_three_mask_kinds_serialize_lowercase_and_spotlight_is_not_a_mask` - all four `EffectKind` variants round-trip through their lowercase wire string; `is_mask` agrees with "not Spotlight" for each.
- `a_mask_round_trips_its_rect_strength_and_roundness_through_a_doc` - a Blur region's `rect`/`strength`/`roundness` survive a full `EditDoc` JSON round trip.
- `absent_mask_fields_are_not_written` - a Spotlight region (all three mask fields `None`) serializes with no `rect`/`strength`/`roundness` keys at all.
- `the_mask_defaults_are_the_spec_numbers` - pins `DEFAULT_BLUR`/`DEFAULT_PIXEL`/`DEFAULT_MASK_ROUNDNESS`/`DEFAULT_MASK_FEATHER` to the spec's numbers.

## DEFAULT_BLUR

```rust
pub const DEFAULT_BLUR: f32 = 0.020;
```

The default blur radius (fraction of output height) when a Blur mask's `strength` is absent.

## DEFAULT_PIXEL

```rust
pub const DEFAULT_PIXEL: f32 = 0.018;
```

The default pixel-cell size (fraction of output height) when a Pixelate mask's `strength` is absent.

## DEFAULT_MASK_ROUNDNESS

```rust
pub const DEFAULT_MASK_ROUNDNESS: f32 = 0.06;
```

The default corner rounding (fraction of the mask rect's short side) when a mask's `roundness` is absent.

## DEFAULT_MASK_FEATHER

```rust
pub const DEFAULT_MASK_FEATHER: f32 = 0.010;
```

The default soft-edge width (fraction of output height) when a mask's `feather` is absent.
