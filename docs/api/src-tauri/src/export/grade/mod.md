# src-tauri/src/export/grade/mod.rs

Spec 3.2 and 3.3 (`docs/superpowers/specs/2026-09-15-editor-parity-features-design.md`): resolves the four numbers `settings::grade::GradeSettings` stores into the eleven `GradeParams` the pixel transform takes, and defines that transform. `settings/grade.rs` is the model; this module is the only place that knows what a preset name means.

The whole module is pure and stateless. A grade is constant for an entire export, so `params_of` is called once in `render::render_edit::EditState::load` and the result is carried on `FrameRenderer`, never rebuilt per frame. `gradedraw` (the sibling module declared here) is the CPU per-pixel loop over a BGRA buffer; the GPU path is `export/fx/fx_grade.wgsl`, fed by `export/fx/fx_uniforms::pack_grade`.

There is no linearisation anywhere in this module. The compositor works in sRGB-encoded BGRA throughout and the preview's canvas is sRGB, so a linearising grade would need a second round trip to match the preview mirror, and would still land on different bytes.

## LUMA_R

```rust
pub const LUMA_R: f32 = 0.2126;
```

The Rec.709 red luma weight, used by `apply_px`'s saturation step. Shared with `fx_grade.wgsl` (`G_LUMA.x`) and `src/editor/stage/grade/gradeParams.ts` (`LUMA[0]`); `gradeShader.test.ts` pins that the GLSL source still carries this literal.

## LUMA_G

```rust
pub const LUMA_G: f32 = 0.7152;
```

The Rec.709 green luma weight. See `LUMA_R`.

## LUMA_B

```rust
pub const LUMA_B: f32 = 0.0722;
```

The Rec.709 blue luma weight. See `LUMA_R`.

## VIGN_IN

```rust
pub const VIGN_IN: f32 = 0.45;
```

The vignette's inner radius as a fraction of the corner distance. Inside it the vignette is exactly zero, so the middle 45 percent of the frame is never darkened and a vignette reads as a corner effect rather than an overall dimming. Read by `vignette_k`.

## CORNER

```rust
pub const CORNER: f32 = 0.70710678;
```

`sqrt(2) / 2`, the distance from the centre of the unit square to a corner in the `u`/`v` space `apply_px` takes. Dividing by it normalises the radial distance so that `vignette_k` is exactly 1 at a corner whatever the aspect ratio, which is what makes a vignette look the same on 16:9 and on 9:16.

## TEMP_GAIN

```rust
pub const TEMP_GAIN: f32 = 0.25;
```

How far a temperature of +/-1 pushes the red and blue channels apart: red is scaled by `1 + TEMP_GAIN * temp` and blue by `1 - TEMP_GAIN * temp`. A quarter, so the widest preset temperature (Frost at -0.22) is a 5.5 percent channel split rather than a colour cast that cannot be undone.

## TINT_GAIN

```rust
pub const TINT_GAIN: f32 = 0.20;
```

How far a tint of +/-1 pushes the green channel: green is scaled by `1 + TINT_GAIN * tint`. Smaller than `TEMP_GAIN` because the eye is most sensitive to green, so the same numeric move reads as a larger shift.

## GradeParams

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradeParams {
    pub exposure: f32,
    pub contrast: f32,
    pub vignette: f32,
    pub saturation: f32,
    pub temp: f32,
    pub tint: f32,
    pub lift: [f32; 3],
    pub gamma: [f32; 3],
    pub gain: [f32; 3],
}
```

The eleven parameters `apply_px` takes, one resolved look. The first three come from the document (`GradeSettings`) and the user can bend them; the other eight are a fixed lookup by preset name and are never stored, so a preset's character cannot drift between two projects that picked the same look.

`lift`, `gamma` and `gain` are per channel in RGB order. `Copy`, because it is carried by value through `FxState` and `FrameRenderer` and is 44 bytes.

### Used by

- `src-tauri/src/export/fx/fx_state.rs` (`FxState.grade`) - the resolved look the FX pass draws
- `src-tauri/src/export/fx/fx_uniforms.rs` (`pack_grade`) - packed into `FxU.grade`'s six vec4 for the GPU path
- `src-tauri/src/export/grade/gradedraw.rs` (`draw_grade`) - the CPU path's per-pixel loop
- `src-tauri/src/export/render/render_edit.rs` (`EditState.grade`) - resolved once per export

## seed_of

```rust
pub fn seed_of(p: GradePreset) -> (f32, f32, f32)
```

The `exposure`, `contrast` and `vignette` a pick WRITES into the document, in that order.

These three numbers are ABSOLUTE, not offsets from anything. Picking a preset in the UI copies this triple into `GradeSettings`; from then on the document owns them and the user may bend any of them without changing which preset is recorded. That is M3's "presets are starting points" contract: the picker keeps showing what was picked until the numbers drift away from this row, at which point the UI reads Custom. Nothing in the model enforces the two staying in sync, and nothing should: a bent Noir is still a Noir the user started from.

`GradePreset::None` seeds `(0.0, 1.0, 0.0)`, which is the identity, so picking None is how a look is removed.

### Used by

- `src/editor/panels/background/GradeSection.tsx` (through the TS mirror `seedOf`) - the Look picker writes all four fields in one save

## params_of

```rust
pub fn params_of(s: &GradeSettings) -> Option<GradeParams>
```

Resolves a document's grade settings into the full eleven parameters: the three stored numbers verbatim, the other eight looked up from the preset table of spec 3.2.

Returns `None` when `s.is_identity()`, which is the point of the `Option`. An ungraded project must not force the FX pass to run and must not pay a per-pixel cost, so `None` means "there is no look here" rather than "here is a look that happens to do nothing". `fx_state::render` builds a state for `Some` alone, the way it already does for the cursor lens; `fxdraw::CpuFx::apply` and `pack_grade` both branch on it. An identity `GradeParams` would be a byte-identical no-op if it were ever applied (pinned by `the_identity_parameters_are_a_byte_identical_no_op_over_a_noise_frame`), but it is never built.

A bent knob on the `None` look is still a grade: `is_identity` checks all four fields, so an exposure of 0.05 over preset None resolves to parameters and renders.

### Used by

- `src-tauri/src/export/render/render_edit.rs` (`EditState::load`) - the only production call site, once per export

## vignette_k

```rust
pub fn vignette_k(u: f32, v: f32) -> f32
```

The vignette's radial falloff at a pixel, 0 in the middle of the frame and 1 at a corner. `u` and `v` are the pixel's offset from the centre in frame fractions, `(x + 0.5) / ow - 0.5` and `(y + 0.5) / oh - 0.5`, so both run from -0.5 to +0.5.

The distance is normalised by `CORNER` and then remapped so that everything inside `VIGN_IN` is exactly 0. `apply_px` squares the result before scaling by `vignette`, which is what makes the falloff read as a soft corner shadow instead of a hard ring.

## apply_px

```rust
pub fn apply_px(c: [f32; 3], p: &GradeParams, u: f32, v: f32) -> [f32; 3]
```

The whole grade, one pixel. `c` is `[R, G, B]`, 0 to 1, sRGB-encoded in value and linear only in the sense that the code does no transfer function. `u` and `v` are the pixel-centre offsets `vignette_k` documents. The return is `[R, G, B]` clamped to 0..1, which the caller rounds back to bytes.

The eight steps, verbatim from spec 3.3:

```
1. Decode.       c = vec3(R, G, B) / 255.0
2. Exposure.     c = c * exp2(p.exposure)
3. Temp, tint.   c.r *= 1.0 + 0.25 * p.temp
                 c.b *= 1.0 - 0.25 * p.temp
                 c.g *= 1.0 + 0.20 * p.tint
4. Lift/gain,    c = p.lift + (p.gain - p.lift) * c
   then gamma.   c = clamp(c, 0.0, 1.0)
                 c = pow(c, 1.0 / max(p.gamma, 0.001))
5. Contrast.     c = (c - 0.5) * p.contrast + 0.5
6. Saturation.   l = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
                 c = vec3(l) + (c - vec3(l)) * p.saturation
7. Vignette.     u = (x + 0.5) / ow - 0.5, v = (y + 0.5) / oh - 0.5
                 d = sqrt(u*u + v*v) / 0.70710678
                 k = clamp((d - 0.45) / (1.0 - 0.45), 0.0, 1.0)
                 c = c * (1.0 - p.vignette * k * k)
8. Clamp.        c = clamp(c, 0.0, 1.0)
```

Steps 2 and 3 are one expression in the code, because exposure and the channel gains are both plain multiplies on the same value.

**The order is fixed.** Changing it changes every existing graded project, silently, because the document stores parameters and not pixels. The 126 generated rows in `grade_table_tests.rs` exist to make that impossible to do by accident.

**The clamp in step 4 is load bearing.** Without it a lifted, exposed highlight can go negative before `pow`, which is NaN on the GPU and a black pixel that appears only on some hardware.

**Four transcriptions, three of which must agree exactly.** This function is the reference. `src-tauri/src/export/fx/fx_grade.wgsl` (`grade_fx`) and `src/editor/stage/grade/gradeShader.ts` (`gradeFragmentSource`) are line-for-line transcriptions of it and must stay in step. `src/editor/stage/grade/gradeCpu.ts` (`gradePixel`) is a fourth transcription that never draws a pixel: it exists only to be pinned against the table this file generates, which is how the preview's numbers are held to the export's.

### Used by

- `src-tauri/src/export/grade/gradedraw.rs` (`draw_grade`) - the CPU path calls it once per pixel

### Behaviors

- `the_grade_is_pinned_at_the_frame_centre_for_every_preset_and_colour` - 108 generated rows, nine presets across twelve colours at the frame centre, preset major and colour minor. The same rows are retyped in `src/editor/stage/grade/grade.test.ts`.
- `the_vignette_darkens_an_edge_and_a_corner_more_than_the_centre` - 18 generated rows for the two heaviest vignettes at the centre, an edge midpoint and a corner, plus that `vignette_k` is exactly 0 at the centre and exactly 1 at a corner.
- `the_identity_parameters_are_a_byte_identical_no_op_over_a_noise_frame` - 256 colours through identity parameters at an off-centre pixel come back unchanged, which is what lets `params_of` return `None` without a special case in the renderer.
- `the_identity_grade_resolves_to_no_parameters_at_all` - the default and preset None resolve to `None`; a bent knob on None does not.
- `picking_a_preset_seeds_the_three_stored_numbers_from_its_row` - four spot checks of `seed_of` against spec 3.2's table.
- `the_stored_numbers_win_over_the_preset_row` - a Noir with all three stored numbers bent keeps Noir's saturation of zero.

## gradedraw

```rust
pub mod gradedraw;
```

The group's one submodule, and the only thing in the tree that runs `apply_px` over a whole buffer: `draw_grade(out, ow, oh, params)`, the CPU path's per-pixel loop over a BGRA frame, called from `fxdraw::CpuFx::apply` as the second of its six stages. It is a leaf - no other module declares one here, which is why this file is a flat list of constants and functions rather than the overview page `mask/mod.md` is. See `grade/gradedraw.md`.
