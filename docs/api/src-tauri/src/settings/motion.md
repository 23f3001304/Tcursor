# src-tauri/src/settings/motion.rs

The project's ONE motion language (M3, `docs/superpowers/specs/2026-09-15-motion-editor-design.md` section 3). A single curve PAIR, stored on `Settings`, that every newly added zoom, layout segment and camera-move keyframe inherits instead of the `"smooth"` those ops used to hardcode - and that `EditOp::ApplyMotionDefault` stamps onto the regions already placed.

Its own file rather than a struct in `model.rs` for the usual reason in this tree: `model.rs` is at its line budget, and the motion default has enough of a story to justify the space.

**Rust never AUTHORS a curve.** Presets live entirely on the TypeScript side (`src/editor/motion/presets.ts`) because the editor is the only thing that ever writes one; this module stores the two strings it is handed and the add ops forward them through the same `valid_easing` (`edit::ops::region`) every other easing already passes through. That is what keeps `keys(...)`, `spring(...)`, `cubic(...)` and the six named curves one channel with one parser on each side.

## MotionSettings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct MotionSettings {
    pub preset: String,
    pub easing: String,
    pub easing_out: String,
}
```

Fields:

- `preset: String` - the NAME the editor showed when these two strings were written (`"soft"`, `"snappy"`, `"cinematic"`, `"mechanical"`, `"bouncy"`). Provenance for the UI only: nothing in the render path reads it, and the editor's picker derives what to display from the STRINGS (`presetOf`), not from this field, so a hand-edited config or a curve dragged off a preset honestly reads as Custom. *Why store it at all rather than re-derive it:* two presets could one day share a curve pair, and the name the user actually picked is the one that should come back.
- `easing: String` - the ramp INTO a region: a zoom-in, a layout segment's entry fade, a camera move's blend from the previous keyframe.
- `easing_out: String` - the ramp OUT of one: a zoom-out, a layout segment's exit fade. A camera move has no exit ramp of its own (the next keyframe's entry is all that follows it), so `AddCameraMove` reads `easing` only.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.motion`) - persisted in `config.json` and snapshotted into every `EditDoc`
- `src-tauri/src/edit/ops/motion.rs` - the only reader: `for_zoom` / `for_layout` / `for_camera` / `apply_default`
- `src/hud/settings/settings.ts` (`MotionSettings`) - the TypeScript mirror
- `src/editor/shell/settings/MotionSection.tsx` - the only writer (Settings > Motion)

## MotionSettings::default

```rust
impl Default for MotionSettings {
    fn default() -> Self
}
```

Soft: `{ preset: "soft", easing: "smooth", easing_out: "smooth" }`.

### Why Soft is the bare word `"smooth"` and not the equivalent `keys(...)` curve

The spec's table gives Soft as `keys(0 0 0 0 0.333 0 b,1 1 -0.333 0 0 0 b)`, which matches smoothstep to within 1e-3 - and the spec itself says Soft *is* today's curve. Writing the WORD rather than that curve buys two things a 1e-3 match cannot:

1. **Every existing region reads back as Soft.** Every zoom, layout segment and camera move in every project ever recorded carries `"smooth"`. If the default were the keys string, all of them would show as Custom in the new picker until rewritten, and "Apply to all regions" would become a mandatory migration rather than an optional sweep.
2. **The shipped camera trajectory stays bit-identical.** `export::camera::jank_probe_tests::filter`'s fingerprint (`0x58da_34b7_41e9_0fd3`) pins the exact scale/centre path of the probe scene. A curve that agrees with smoothstep only to 1e-3 moves it.

The editor's `PRESETS` table mirrors this exactly (`src/editor/motion/presets.ts`: Soft is `"smooth"` on both ramps), and `presetOf` treats the bare word as Soft, so the two sides agree on what the default is called.

### Behaviors

- `default_is_soft_and_soft_is_the_bare_smooth_word` (`motion_tests.rs`) - pins all three fields, with the reason in the test.
- `a_config_written_before_the_field_loads_soft` - `Settings` parsed from `{}` (and from a config carrying only other keys) lands on `MotionSettings::default()`, not on `String::default()` (empty strings, which `valid_easing` would then have to rescue at every add op).
- `a_chosen_preset_round_trips_through_json` - a Bouncy pair survives a write/read cycle unchanged.
- `partial_motion_json_fills_the_missing_halves_with_soft` - `{"preset":"cinematic"}` still gets real curve strings, because `#[serde(default)]` is on the struct.
