# src-tauri/src/export/fx_uniforms.rs

Packs `FxState` into the `FxU` GPU uniform struct consumed by `fx.wgsl`. Defines the canonical numeric-id mappings for click styles, spotlight modes, and video FX modes - the single source of truth that the `FX_*` constants in `fx.wgsl` must mirror exactly.

## MAX_HITS

```rust
pub const MAX_HITS: usize = 16;
```

Maximum simultaneous click effects packed into the shader uniform. Clicks beyond this limit are silently dropped (oldest first). The WGSL shader declares a fixed-size array of this length, so the value must match between Rust and WGSL.

### Used by

- `src-tauri/src/export/fx_uniforms.rs` - `build_fx_u` uses `.take(MAX_HITS)` when iterating hits.
- `src-tauri/src/export/fx_gpu.rs` - `FxU` embeds `[[f32; 4]; MAX_HITS]`, so its size in bytes determines the uniform buffer allocation.

## FxU

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FxU {
    pub a: [f32; 4],                // ow, oh, style(0..6), hit_count
    pub b: [f32; 4],                // spot_cx, spot_cy, spot_dim*alpha, spot_active(0/1)
    pub c: [f32; 4],                // spot_r_in(px), spot_r_out(px), intensity, _pad
    pub d: [f32; 4],                // spot_mode_id, time_s, _pad, _pad
    pub tint: [f32; 4],             // r, g, b (0..1), _pad
    pub color: [f32; 4],            // r, g, b (0..1), _pad
    pub hits: [[f32; 4]; MAX_HITS], // x, y, progress, _pad
    pub e: [f32; 4],                // video_mode_id, alpha, t, _pad
}
```

The shader uniform block. All fields are `[f32; 4]` (vec4) for std140 alignment. `bytemuck::Pod + Zeroable` enables zero-copy byte casting for GPU upload via `bytemuck::bytes_of`. Fields are in logical RGBA space; the `Bgra8Unorm` format handles byte order on texture sample and store.

### Fields

- `a` - *`[ow, oh, style_id, hit_count]`: output dimensions in pixels, click-style numeric id (0..6), active hit count (capped at `MAX_HITS`). The shader reads `a.z` to select a click-effect branch and `a.w` to bound the hit loop.*
- `b` - *`[spot_cx, spot_cy, dim*alpha, spot_active]`: spotlight center in output pixels, pre-multiplied dim strength, and a 0/1 presence flag. Pre-multiplying dim by alpha saves a per-pixel multiply in the hot shader path.*
- `c` - *`[spot_r_in, spot_r_out, intensity, _pad]`: inner radius in output pixels (`oh * radius_frac`), outer radius (`r_in + oh * feather_frac`, minimum feather 0.001 to prevent divide-by-zero in the shader's feather step), global effect intensity scalar.*
- `d` - *`[spot_mode_id, time_s, _pad, _pad]`: numeric spotlight mode (0..5) and elapsed time in seconds for animated modes.*
- `tint` - *spotlight tint as normalized RGB floats (`0..1`), converted from the `[u8; 3]` tint bytes.*
- `color` - *click-effect color as normalized RGB floats, converted from the `[u8; 3]` color bytes.*
- `hits` - *up to `MAX_HITS` click hits, each `[x, y, progress, 0.0]` in output pixels.*
- `e` - *`[video_mode_id, alpha, time_s, _pad]`: video FX mode (0..3), fade alpha, and elapsed time; all zero when no video FX is active.*

### Used by

- `src-tauri/src/export/fx_gpu.rs` - `GpuFx::apply` uploads `FxU` as a wgpu `UNIFORM` buffer at binding 2.

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

## build_fx_u

```rust
pub fn build_fx_u(state: &FxState, ow: u32, oh: u32) -> FxU
```

Packs a complete `FxState` into an `FxU` ready for GPU upload.

### Inputs

- `state: &FxState` - the renderer-agnostic FX description built by `fx_state_at`. *Why:* this function's sole job is translating the Rust description into the flat numeric layout the shader expects.*
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* radius fractions are multiplied by `oh` to get pixel radii; `ow`/`oh` are also stored in `FxU.a` for the shader's UV arithmetic.*

### Returns

`FxU` - fully populated uniform; unused slots (no spotlight, no video FX, excess hits) are zero-initialized.

### Implementation

1. `style = style_id(state.style)` -> `a[2]`.
2. Copy up to `MAX_HITS` hits: `hits[i] = [h.x, h.y, h.progress, 0.0]`. Hits beyond `MAX_HITS` are silently dropped (overflow is rare; `MAX_HITS=16` exceeds any plausible burst). Count -> `a[3]`.
3. If `state.spot` is `Some(s)`:
   - `r_in = oh * s.radius_frac.max(0.0)`, `r_out = r_in + oh * s.feather_frac.max(0.001)`. *Why `.max(0.001)`:* prevents `r_in == r_out`, which would produce a divide-by-zero in the shader feather step.*
   - `b[2] = dim.clamp(0,1) * alpha.clamp(0,1)`. *Why pre-multiply:* avoids a per-pixel multiply in the hot spotlight path.*
   - `b[3] = 1.0` (spot active).
   - Pack `d = [spot_mode_id(s.mode), s.t, 0, 0]`, tint as `[r/255, g/255, b/255, 0]`.
4. If `state.spot` is `None`: `b` and `d` and `tint` are zero-initialized (`b[3] = 0.0` tells the shader no spotlight).
5. If `state.video` is `Some(v)`: `e = [video_mode_id(v.mode), v.alpha, v.t, 0]`; else `e = [0; 4]`.
6. Normalize `state.color` from `u8` to `f32` per channel -> `color`.

### Behaviors worth knowing

- `maps_style_hits_spot_and_color` - verifies `a` fields, `b[3]=1` (spot active), `b[2]=dim*alpha`, `color[0]=1.0` (red normalized), `hits[0]` layout.
- `no_spot_sets_inactive` - `state.spot = None` -> `b[3] = 0.0`.
- `spot_mode_id_and_tint_pack` - `Nebula` mode -> `d[0]=4.0`, time in `d[1]`, tint channel normalization verified.
