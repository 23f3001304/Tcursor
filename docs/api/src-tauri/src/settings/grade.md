# src-tauri/src/settings/grade.rs

Spec 3.1 (`docs/superpowers/specs/2026-09-15-editor-parity-features-design.md`): a colour grade is a property of the whole project, not of a moment in it, so it lives on `Settings` rather than as a region like a mask or a zoom. Picking one of the nine named presets WRITES three ABSOLUTE numbers - `exposure`, `contrast`, `vignette` - into the doc; the preset's other eight parameters (saturation, temperature, tint, and the per-channel lift/gamma/gain triples) stay a fixed lookup by `preset` name, never stored here - Batch 2b adds that table in `export/grade`. `GradeSettings::default()` is the identity, so every document written before this field existed loads it unchanged and exports byte-identically until a look is actually chosen.

Its own file rather than a struct in `model.rs` for the usual reason in this tree: `model.rs` is at its line budget.

## GradePreset

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GradePreset {
    #[default]
    None,
    Cinematic,
    Noir,
    Vintage,
    Frost,
    Golden,
    Midnight,
    Vivid,
    Dreamy,
}
```

The nine names the preset table (spec 3.2) is keyed by, serialised lowercase for the wire. `None` is the default and the only one with no visual effect. `GradeSettings.preset` records the last one picked; Batch 2b's `export/grade` will be the first thing that looks the other eight up by this name to resolve the fixed parameters the table gives them.

### Used by

- `src-tauri/src/settings/grade.rs` (`GradeSettings.preset`) - which row of the preset table a doc has picked
- `src/hud/settings/settings.ts` (`GradePreset`) - the TypeScript mirror

## GradeSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct GradeSettings {
    pub preset: GradePreset,
    pub exposure: f32,
    pub contrast: f32,
    pub vignette: f32,
}
```

Fields:

- `preset: GradePreset` - the last look picked. Default `None`. Provenance for the UI picker only: choosing a preset writes the three fields below from its row in the table, and bending any of them away from that row is what makes the picker read Custom (spec 3.1's "presets are starting points" contract) - nothing in this struct enforces the two staying in sync.
- `exposure: f32` - stops, `-2.0` to `+2.0`. Default `0.0`. Applied as `exp2(exposure)` (spec 3.3 step 2).
- `contrast: f32` - multiplier about a 0.5 pivot, `0.5` to `1.8`. Default `1.0`. Applied as `(c - 0.5) * contrast + 0.5` (spec 3.3 step 5).
- `vignette: f32` - `0` to `1`, how dark the corners go. Default `0.0`. Radial falloff from 0 at the centre to this value at a corner (spec 3.3 step 7).

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.grade`) - persisted in `config.json`
- `src/hud/settings/settings.ts` (`GradeSettings`) - the TypeScript mirror

Nothing reads it yet: Batch 2b's `export/grade` is the first consumer, resolving a full `GradeParams` (the eleven-parameter set spec 3.3's formula takes) from this struct plus the preset table.

## GradeSettings::default

```rust
impl Default for GradeSettings {
    fn default() -> Self
}
```

The identity: `{ preset: None, exposure: 0.0, contrast: 1.0, vignette: 0.0 }`. *Why:* a config written before this field existed must deserialize to exactly this value (via the struct's own `#[serde(default)]`) and render exactly as it did before the grade pass existed - see `is_identity`.

## GradeSettings::is_identity

```rust
pub fn is_identity(&self) -> bool
```

True when all four fields still match `default()`. Batch 2b skips the grade pass when true, so an ungraded document pays no per-pixel cost and produces exactly today's bytes.

### Behaviors

- `the_default_is_the_identity` - pins the four default values and that `is_identity()` is true for them, false for a `vignette` nudged off zero.
- `a_config_written_before_the_field_loads_the_identity` - `Settings` parsed from `{}` lands on `GradeSettings::default()`; a config carrying only `{"grade":{"preset":"noir"}}` still fills `contrast` with `1.0`.
- `every_preset_name_is_lowercase_on_the_wire` - all nine `GradePreset` variants round-trip through their lowercase JSON string.
- `a_chosen_grade_round_trips_through_settings` - a full `GradeSettings` survives a `Settings` write/read cycle unchanged.
