# src-tauri/src/export/fx/fx_uniforms.rs

Packs `FxState` into the `FxU` GPU uniform struct consumed by `fx.wgsl` (and by its concatenated other half, `fx_clicks.wgsl`). Defines the canonical numeric-id mappings for click styles, spotlight modes, and video FX modes - the single source of truth that the `FX_*` constants in `fx.wgsl` must mirror exactly - plus the hue rotation Neon's second tube is drawn in.

## MAX_HITS

```rust
pub const MAX_HITS: usize = 16;
```

Maximum simultaneous click effects packed into the shader uniform. Clicks beyond this limit are silently dropped (oldest first). The WGSL shader declares a fixed-size array of this length, so the value must match between Rust and WGSL.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` uses `.take(MAX_HITS)` when iterating hits.
- `src-tauri/src/export/fx/fx_gpu.rs` - `FxU` embeds `[[f32; 4]; MAX_HITS]`, so its size in bytes determines the uniform buffer allocation.

## MAX_MASKS

```rust
pub const MAX_MASKS: usize = 8;
```

How many masks one frame can carry on the GPU. The `mask` block is `3 * MAX_MASKS` vec4, and `fx_mask.wgsl`'s `mask_fx` loops over exactly this many slots, so the two must match: the WGSL side spells it as the literal `8` in its loop bound and `24` in the `fx.wgsl` struct, and `the_mask_block_is_three_vec4_per_mask_and_the_grade_block_still_trails_it` pins this constant to 8 so a change here fails the test rather than reading past the array on one side only.

*What happens to a ninth mask:* `fx_state::render` splits the list, keeps the first eight for the uniform and paints the rest through `maskdraw::draw_masks` on the CPU before the renderer runs. So the cap degrades rather than dropping anything, and the spilled masks sit UNDER the kept ones.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `pack_masks` uses `.take(MAX_MASKS)`.
- `src-tauri/src/export/fx/fx_state.rs` - `render` computes the spill against it.

## FxU

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FxU {
    pub a: [f32; 4],                // ow, oh, style(0..6), hit_count
    pub b: [f32; 4],                // spot_cx, spot_cy, spot_dim*alpha, spot_active(0/1)
    pub c: [f32; 4],                // spot_r_in(px), spot_r_out(px), intensity, _pad
    pub d: [f32; 4],                // spot_mode_id, time_s, keep_camera_lit(0/1), cam_radius(px)
    pub tint: [f32; 4],             // r, g, b (0..1), _pad
    pub color: [f32; 4],            // r, g, b (0..1), _pad
    pub color2: [f32; 4],           // `color` rotated NEON_HUE_SHIFT of hue (Neon's 2nd tube), _pad
    pub hits: [[f32; 4]; MAX_HITS], // x, y, progress, _pad
    pub e: [f32; 4],                // video_mode_id, alpha, t, _pad
    pub cam: [f32; 4],              // camera-exclusion rect (px): min_x, min_y, max_x, max_y
    // ... then the six glass-cursor slots (see `FxU::lens_a`), and last:
    pub mask: [[f32; 4]; 3 * MAX_MASKS], // three vec4 per mask, eight masks (see `FxU::mask`)
    pub grade: [[f32; 4]; 6],       // the colour grade's eleven parameters (see `FxU::grade`)
}
```

The shader uniform block. All fields are `[f32; 4]` (vec4) for std140 alignment. `bytemuck::Pod + Zeroable` enables zero-copy byte casting for GPU upload via `bytemuck::bytes_of`. Fields are in logical RGBA space; the `Bgra8Unorm` format handles byte order on texture sample and store.

### Fields

- `a` - *`[ow, oh, style_id, hit_count]`: output dimensions in pixels, click-style numeric id (0..6), active hit count (capped at `MAX_HITS`). The shader reads `a.z` to select a click-effect branch and `a.w` to bound the hit loop.*
- `b` - *`[spot_cx, spot_cy, dim*alpha, spot_active]`: spotlight center in output pixels, pre-multiplied dim strength, and a 0/1 presence flag. Pre-multiplying dim by alpha saves a per-pixel multiply in the hot shader path.*
- `c` - *`[spot_r_in, spot_r_out, intensity, _pad]`: inner radius in output pixels (`oh * radius_frac`), outer radius (`r_in + oh * feather_frac`, minimum feather 0.001 to prevent divide-by-zero in the shader's feather step), global effect intensity scalar.*
- `d` - *`[spot_mode_id, time_s, keep_camera_lit, cam_radius]`: numeric spotlight mode (0..5), elapsed time in seconds for animated modes, a 0/1 flag meaning "undo the dim inside `cam`" (set when `Spot::dim_camera` is `false`, i.e. the "don't dim the webcam" option), and the camera panel's corner radius in output pixels.*
- `tint` - *spotlight tint as normalized RGB floats (`0..1`), converted from the `[u8; 3]` tint bytes.*
- `color` - *click-effect color as normalized RGB floats, converted from the `[u8; 3]` color bytes.*
- `color2` - *the same colour rotated `NEON_HUE_SHIFT` degrees of hue, already normalized. Read only by `fx_clicks.wgsl`'s Neon branch, for its second tube. Zero-cost for every other style, and it keeps an RGB->HSV->RGB round trip out of the per-fragment path.*
- `hits` - *up to `MAX_HITS` click hits, each `[x, y, progress, 0.0]` in output pixels.*
- `e` - *`[video_mode_id, alpha, time_s, _pad]`: video FX mode (0..3), fade alpha, and elapsed time; all zero when no video FX is active.*
- `cam` - *`[min_x, min_y, max_x, max_y]`: the active camera panel's rect in output pixels (`Spot::cam_rect`, itself from `scene.camera.rect`). The shader's `rrect_cov` helper tests pixels against this rect + `d.w`'s radius to build the un-dim mask; zero when there is no active spot.*
- `mask` - *eight mask slots of three vec4 each, filled by `pack_masks`. See `FxU::mask`.*
- `grade` - *the resolved colour grade, six vec4 written by `pack_grade` and read by `fx_grade.wgsl`. All zero, with the active flag clear, when the project has no look. See `FxU::grade`.*

### Used by

- `src-tauri/src/export/fx/fx_gpu.rs` - `GpuFx::apply` uploads `FxU` as a wgpu `UNIFORM` buffer at binding 2.
- `src-tauri/src/export/fx/fx.wgsl` - declares the mirrored WGSL `struct FxU` with the identical field layout; `d.z`/`d.w`/`cam` drive the `camcov` un-dim mix immediately after the spotlight block.

### FxU::lens_a

```rust
pub lens_a: [f32; 4],  // sprite lens box (px): centre x, centre y, w, h
pub lens_b: [f32; 4],  // busy angle (rad), on, click squash, ink progress (<0 = none)
pub lens_c: [f32; 4],  // ink origin (px): x, y, _pad, _pad
pub back_a: [f32; 4],  // cursor-back rounded rect (px): min_x, min_y, max_x, max_y
pub back_b: [f32; 4],  // corner radius (px), on, click squash, ink progress
pub back_c: [f32; 4],  // ink origin (px): x, y, ring-instead-of-drop, _pad
```

The six slots the glass cursor material (`fx_lens.wgsl`) reads. `lens_b[1]` and `back_b[1]` are the on/off flags - the shader's first test in each of `lens_fx` and `lens_back` - so an absent shape costs one uniform compare, not a branch on geometry.

`build_fx_u` fills all six from `FxState::lens`, zeroing them when it is `None`. They must stay in the same order as the `FxU` struct in `fx.wgsl`, like every other field here.

The lens MASK does not live in the uniform: it is a second texture, bound at group 0 binding 3 and cached by `GpuFx` (see `fx_gpu.md`).

### FxU::lens_b

See `FxU::lens_a`.

### FxU::lens_c

See `FxU::lens_a`.

### FxU::back_a

See `FxU::lens_a`.

### FxU::back_b

See `FxU::lens_a`.

### FxU::back_c

See `FxU::lens_a`.

### FxU::mask

```rust
pub mask: [[f32; 4]; 3 * MAX_MASKS],  // 24 vec4: three per mask, eight masks
pub grade: [[f32; 4]; 6],             // 6 vec4: the colour grade's eleven parameters
```

Two blocks at the END of the struct, after the lens slots, and **both are live**: `build_fx_u` fills `mask` through `pack_masks` and `fx_mask.wgsl` reads it, and it fills `grade` through `pack_grade` and `fx_grade.wgsl` reads it (see `FxU::grade` for the grade's slot table). They bought the parity features room to land one at a time, each through its own packing function, so neither had to edit this struct's shape or re-check its layout against the shader - which is exactly what happened: the masks and the grade were built in parallel and each filled its own block.

`the_mask_block_is_three_vec4_per_mask_and_the_grade_block_still_trails_it` pins their OFFSETS, not just the total size: `offset_of!(FxU, mask)` is `16 * (7 + MAX_HITS + 8)` and `offset_of!(FxU, grade)` is `16 * (7 + MAX_HITS + 8 + 3 * MAX_MASKS)`, so inserting or removing a field ANYWHERE ahead of them fails the test instead of silently sliding what each packing function writes. Size alone cannot see a field swapped for another of the same width. **Resizing `mask` moved `grade` 128 bytes later**, and the `fx.wgsl` struct was widened from `array<vec4<f32>, 16>` to `array<vec4<f32>, 24>` in the same commit for exactly that reason.

*Why reserve rather than add later:* the uniform is two declarations that must agree byte for byte - this one and `struct FxU` in `fx.wgsl` - and the only thing checked across that seam is the bound buffer's SIZE. The FX bind group declares `min_binding_size: None` (`fx_gpu_pipeline.rs`), so nothing validates field ORDER: swap two `vec4`s on one side only and every GPU test still passes while the shader reads the wrong numbers. **Each block therefore pins its own packing offsets with a test** - a slot-by-slot assertion on what `build_fx_u` writes, next to these two, one for `pack_masks` and one for `pack_grade` - because no GPU test will catch a disagreement for them. Growing the tail once, with both sides moved together and the GPU tests green, costs nothing (the struct is 61 vec4 - 7 head + `MAX_HITS` 16 + 8 (`e`, `cam` and the six lens slots) + 24 mask + 6 grade - so 976 bytes against a 64 KiB uniform limit, and `fx_gpu.rs` sizes the buffer from `bytemuck::bytes_of(&u)`, so it follows the struct on its own); growing it twice more, concurrently, from two agents editing the same two files, is where the layouts drift apart.

WGSL's `array<vec4<f32>, N>` and `#[repr(C)]`'s `[[f32; 4]; N]` agree - 16-byte stride, no padding between elements - so the two arrays mirror across the seam like every other field here.

### FxU::grade

```rust
pub grade: [[f32; 4]; 6],
```

The colour grade's eleven parameters, written by `pack_grade` and read by `grade_fx` in `fx_grade.wgsl`. Slot by slot:

| slot | contents |
|---|---|
| `grade[0]` | `exposure`, `contrast`, `saturation`, `vignette` |
| `grade[1]` | `temp`, `tint`, `active` (0 or 1), pad |
| `grade[2]` | `lift.rgb`, pad |
| `grade[3]` | `gamma.rgb`, pad |
| `grade[4]` | `gain.rgb`, pad |
| `grade[5]` | spare, always zero |

`grade[0]` is deliberately not in the struct's field order: it pairs the four numbers the shader reads as one `vec4` in its first two lines, which keeps the hot branch to one load. `active` is the flag `grade_fx` branches on, so an ungraded project costs exactly one uniform compare per fragment and no arithmetic.

See `FxU::mask` for why this block sits at the tail and what it costs.

*The layout is pinned by index, not by type.* The FX bind group declares `min_binding_size: None`, so only the buffer's SIZE crosses the seam: swap two slots on one side and every GPU test still passes while the shader reads the wrong numbers. `pack_grade_lays_out_the_eleven_parameters_and_flags_the_inactive_case` asserts each of the six slots by value, and `fx_grade.wgsl` is edited in the same commit as `pack_grade`, which together are the only guard there is.

## style_id

```rust
pub fn style_id(s: ClickFxStyle) -> f32
```

Maps a `ClickFxStyle` variant to the numeric id the shader reads from `FxU.a[2]`. This is the single source of truth - the `FX_*` constants in `fx.wgsl` must equal these values.

### Inputs

- `s: ClickFxStyle` - the click-effect variant from user settings. *Why:* the shader cannot accept a Rust enum; it reads a float and selects a branch.*

### Returns

`f32` - `None=0.0`, `Ripple=1.0`, `Pulse=2.0`, `Glow=3.0`, `Shockwave=4.0`, `Particles=5.0`, `Neon=6.0`.

### Behaviors worth knowing

- `style_id_covers_all_variants` - asserts every variant maps to its expected id, preventing silent drift if the enum is reordered.

## video_mode_id

```rust
pub fn video_mode_id(m: VideoFxMode) -> f32
```

Maps a `VideoFxMode` variant to the shader id stored in `FxU.e[0]`.

### Inputs

- `m: VideoFxMode` - the video FX variant. *Why:* same reason as `style_id` - the shader reads a float, not a Rust enum.*

### Returns

`f32` - `NebulaWash=0.0`, `CinematicDim=1.0`, `ScreenFocus=2.0`, `ColorPop=3.0`.

## spot_mode_id

```rust
pub fn spot_mode_id(m: SpotlightMode) -> f32
```

Maps a `SpotlightMode` variant to the shader id stored in `FxU.d[0]`.

### Inputs

- `m: SpotlightMode` - the spotlight mode. *Why:* the shader selects a spotlight rendering branch from this float.*

### Returns

`f32` - `Classic=0.0`, `Blur=1.0`, `Halo=2.0`, `Breathing=3.0`, `Nebula=4.0`, `Vignette=5.0`.

## pack_masks

```rust
pub fn pack_masks(masks: &[MaskDraw]) -> [[f32; 4]; 3 * MAX_MASKS]
```

Lays the first `MAX_MASKS` masks into the uniform's `mask` block, three vec4 per slot. Slot `i` occupies:

| Index | Contents |
|---|---|
| `mask[3i]` | `[min_x, min_y, max_x, max_y]` - the projected rect in output pixels |
| `mask[3i + 1]` | `[radius_px, feather_px, amount_px, kind_id]` - the shape and the strength |
| `mask[3i + 2]` | `[dim, alpha, 0, 0]` - the highlight darkness and this region's fade |

**A slot whose `kind_id` is 0 is empty**, which is why there is no count field beside the block. A count is a second number that has to stay in step with the array, and `mask_fx` would have to read it before every slot anyway; a per-slot marker cannot go stale. Unused slots are left as zeros, so `kind_id` there is 0 for free (`pack_masks_lays_out_three_vec4_per_slot_and_caps_at_eight` pins that).

More than `MAX_MASKS` masks are not an error here: the extra ones are simply not packed, because `fx_state::render` has already split them off and painted them on the CPU.

**Why the layout is pinned by tests and by nothing else.** The FX bind group declares `min_binding_size: None` (`fx_gpu_pipeline.rs`), so the only thing that crosses the Rust-to-WGSL seam is the buffer's SIZE. Swap two vec4s on one side only, or reorder the three fields inside a slot, and every GPU test still passes while `mask_fx` reads the radius as a coordinate. The `offset_of!` assertions on `FxU` and the index-by-index assertions on this function's output are the whole guard, which is why `fx_mask.wgsl` is edited in the same commit as this file.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u`, as the `mask:` line of its returned literal.

## build_fx_u

```rust
pub fn build_fx_u(state: &FxState, ow: u32, oh: u32) -> FxU
```

Packs a complete `FxState` into an `FxU` ready for GPU upload.

### Inputs

- `state: &FxState` - the renderer-agnostic FX description built by `fx_state_at`. *Why:* this function's sole job is translating the Rust description into the flat numeric layout the shader expects.*
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* radius fractions are multiplied by `oh` to get pixel radii; `ow`/`oh` are also stored in `FxU.a` for the shader's UV arithmetic.*

### Returns

`FxU` - fully populated uniform; unused slots (no spotlight, no video FX, excess hits, mask slots past the live ones, and the whole `grade` block on an ungraded project) are zero-initialized.

### Implementation

1. `style = style_id(state.style)` -> `a[2]`.
2. Copy up to `MAX_HITS` hits: `hits[i] = [h.x, h.y, h.progress, 0.0]`. Hits beyond `MAX_HITS` are silently dropped (overflow is rare; `MAX_HITS=16` exceeds any plausible burst). Count -> `a[3]`.
3. If `state.spot` is `Some(s)`:
   - `r_in = oh * s.radius_frac.max(0.0)`, `r_out = r_in + oh * s.feather_frac.max(0.001)`. *Why `.max(0.001)`:* prevents `r_in == r_out`, which would produce a divide-by-zero in the shader feather step.*
   - `b[2] = dim.clamp(0,1) * alpha.clamp(0,1)`. *Why pre-multiply:* avoids a per-pixel multiply in the hot spotlight path.*
   - `b[3] = 1.0` (spot active).
   - Pack `d = [spot_mode_id(s.mode), s.t, keep, s.cam_radius]` where `keep = if s.dim_camera { 0.0 } else { 1.0 }`, tint as `[r/255, g/255, b/255, 0]`, and `cam = s.cam_rect`.
4. If `state.spot` is `None`: `b`, `d`, `tint`, and `cam` are all zero-initialized (`b[3] = 0.0` tells the shader no spotlight; `d[2] = 0.0` also means the camera-keep mix is a no-op since there is nothing to exclude).
5. If `state.video` is `Some(v)`: `e = [video_mode_id(v.mode), v.alpha, v.t, 0]`; else `e = [0; 4]`.
6. Normalize `state.color` from `u8` to `f32` per channel -> `color`, and `hue_shift(state.color, NEON_HUE_SHIFT)` -> `color2`. *Why unconditionally rather than only for Neon:* it is one HSV round trip per frame, and a branch here would be one more place the uniform could disagree with what the shader assumes is always populated.

### Behaviors worth knowing

- `maps_style_hits_spot_and_color` - verifies `a` fields, `b[3]=1` (spot active), `b[2]=dim*alpha`, `color[0]=1.0` (red normalized), `hits[0]` layout, and that `color2` carries pure red rotated to orange `(1, 0.5, 0)`.
- `no_spot_sets_inactive` - `state.spot = None` -> `b[3] = 0.0`.
- `spot_mode_id_and_tint_pack` - `Nebula` mode -> `d[0]=4.0`, time in `d[1]`, tint channel normalization verified.
- `dim_camera_false_sets_keep_flag_and_cam_rect` - `Spot::dim_camera = false` -> `d[2] = 1.0`, `d[3]` equals `cam_radius`, `cam` equals `cam_rect` verbatim.
- `dim_camera_true_clears_keep_flag` - `Spot::dim_camera = true` (today's default) -> `d[2] = 0.0`.
- `the_mask_block_is_three_vec4_per_mask_and_the_grade_block_still_trails_it` - with no masks and no grade in the state every slot of both blocks comes back zero, `MAX_MASKS` is 8, `size_of::<FxU>()` is `16 * (7 + MAX_HITS + 8 + 3 * MAX_MASKS + 6)` = 976 bytes, and `offset_of!` puts `mask` at `16 * (7 + MAX_HITS + 8)` = 496 and `grade` `3 * MAX_MASKS` vec4 after it, at 880 - the arithmetic pins the two blocks to the tail by position, so a field added, removed or reordered ahead of them (which would silently shift what the shader reads) fails the test rather than the pixels.

## pack_grade

```rust
pub fn pack_grade(g: Option<&crate::export::grade::GradeParams>) -> [[f32; 4]; 6]
```

Packs a resolved colour grade into `FxU.grade`'s six vec4. `None` gives `[[0.0; 4]; 6]`, which leaves the active flag at `grade[1][2]` clear so `grade_fx` returns the colour untouched after a single compare. The layout table is in `FxU::grade`.

`build_fx_u` calls it as `pack_grade(state.grade.as_ref())`, so an ungraded project writes zeros exactly as it did before the grade existed. There is no separate "grade enabled" flag anywhere in the pipeline: `export::grade::params_of` returns `None` for the identity, `FxState.grade` carries that `None` through, and the zero block is the end of the chain.

**`min_binding_size: None`.** The FX bind group does not declare a binding size (`fx_gpu_pipeline.rs`), so wgpu validates only the buffer's total SIZE against the shader, never its field ORDER. A slot written here and read at a different index in `fx_grade.wgsl` produces wrong pixels with every GPU test still green. The guards are the index-by-index assertion in `pack_grade_lays_out_the_eleven_parameters_and_flags_the_inactive_case` and the rule that this function and the shader are edited in one commit.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` (`build_fx_u`) - fills `FxU.grade`
- `src-tauri/src/export/fx/fx_grade.wgsl` (`grade_fx`) - the consumer, slot for slot

### Behaviors

- `pack_grade_lays_out_the_eleven_parameters_and_flags_the_inactive_case` - `None` gives six zero vec4 with the active flag clear; a filled `GradeParams` lands in the exact six slots the table names, with the spare slot still zero.

## NEON_HUE_SHIFT

```rust
pub const NEON_HUE_SHIFT: f32 = 30.0;
```

Degrees of hue Neon's second tube is rotated from the user's tint.

*Why the rotation lives in Rust and not in the shader:* an RGB->HSV->RGB round trip is a dozen lines of branchy WGSL run per fragment for a value that changes once per frame. Here it is one `build_fx_u` call, and `clickdraw.rs` gets the identical number for free instead of porting the shader version - which is what keeps the CPU fallback's second tube the same colour as the GPU's.

### Behaviors

- `neon_hue_shift_is_pinned_at_30_degrees` - the constant both `fx_clicks.wgsl`'s comment and `clickdraw.rs` cite.

## hue_shift

```rust
pub fn hue_shift(rgb: [u8; 3], deg: f32) -> [f32; 3]
```

Rotates `rgb` by `deg` degrees of hue, keeping saturation and value, returning 0..1 components.

### Inputs

- `rgb: [u8; 3]` - the user's click-FX tint. *Why bytes in, floats out:* bytes are what `ClickFxSettings` stores, and floats are what both the uniform and `clickdraw.rs`'s re-quantisation want.
- `deg: f32` - degrees to rotate, wrapped into 0..360 (so a negative or >360 rotation is well defined).

### Returns

`[f32; 3]` in 0..1.

### Implementation

Standard RGB -> HSV -> RGB, with the hue sector arithmetic written out rather than pulled in from a crate. A grey input has `chroma == 0`, so saturation is 0 and the rotation is a no-op - it cannot pick up a hue it never had.

### Behaviors

- `hue_shift_rotates_and_wraps` - red +30 is orange `(1, 0.5, 0)`, red +120 is green, blue +120 wraps past 360 back to red, +0 is a no-op, and mid-grey and black come back unchanged.

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` fills `FxU.color2` with it.
- `src-tauri/src/export/fx/click/clickdraw.rs` - `draw_clicks`' `Neon` arm, for the CPU fallback's second tube.

