# src-tauri/src/export/fx/fx_lensdraw.rs

The CPU stand-in for `fx_lens.wgsl`, reached when no wgpu adapter is available (`fx_state::select_fx` falls back to `fxdraw::CpuFx`, which calls `draw_lens` last - matching the shader's place at the end of `fs_main`).

**What it mirrors, and the one trick that makes it possible.** The MAGNIFICATION (`fx_lens::ZOOM`, the readable part of the glass - owner ruling 2026-09-14), the drop shadow, the lift + cool cast, and the click ink drop or over-text ring. The shader RE-SAMPLES the frame through a displaced UV; this path composites in place over the same buffer, which used to mean there was no undisturbed source left to re-sample (the trade `clickdraw.rs` still makes for Shockwave's chromatic band). `snapshot` now copies the shape's box aside before anything in it is touched, and `magnify` re-samples THAT - the box is a few thousand pixels, so the copy costs nothing next to the per-pixel work already being done.

**THE DELIBERATE DIFFERENCES.** No rim bend (`LENS_DISP`), no rim frost, no rim light. A glass cursor on a CPU-only machine is a clean magnifier without the edge glint. Nothing moves or is placed differently - the geometry is identical, because it comes from the same `Lenses` the shader would have used - and the text under it reads exactly as it does on the GPU.

## LIFT

```rust
const LIFT: [f32; 3] = [1.05, 1.08, 1.14];
```

The lift + cool cast inside a shape, per RGB channel - the shader's `col * vec3(1.05, 1.08, 1.14)`, applied to the magnified sample in `magnify`. A multiply, not a wash: it replaced a 22% near-white blend that lightened black letters toward grey, which cost the contrast the magnifier had just bought.

## SHADOW

```rust
const SHADOW: f32 = 0.25;
const DROP: f32 = 2.0;
const INK: f32 = 0.30;
```

Mirrors `fx_lens.wgsl`'s `LENS_SHADOW`, `LENS_DROP` and `LENS_INK` exactly, so the two paths' shadows and ink drops land in the same place at the same strength.

## DROP

See `SHADOW`.

## INK

See `SHADOW`.

## over

```rust
fn over(out: &mut [u8], i: usize, c: [f32; 3], a: f32)
```

Alpha-blend RGB `c` over the BGRA pixel at byte index `i`. The `clickdraw::blend` primitive, kept local so the two files' gains stay independently tunable.

## darken

```rust
fn darken(out: &mut [u8], i: usize, f: f32)
```

Multiply the pixel at `i` down by `f` (0 = black, 1 = untouched) - the drop shadow, which is a multiply rather than a blend so it darkens whatever is under it instead of tinting it toward one colour.

## for_box

```rust
fn for_box(ow: u32, oh: u32, b: [f32; 4], mut f: impl FnMut(usize, f32, f32))
```

Visit every pixel of the axis-aligned box `(x0, y0)..(x1, y1)`, clipped to the frame, with its pixel centre. Keeps the per-pixel work off the rest of the frame, the same role `clickdraw::for_disc` plays there.

## rr_sd

```rust
fn rr_sd(px: f32, py: f32, mn: [f32; 2], mx: [f32; 2], r: f32) -> f32
```

Signed distance to a rounded rect - `fx_lens.wgsl::lens_rr_sd` line for line, so the CPU disc/pill/bar has the same edge the shader's does.

## ink_cov

```rust
fn ink_cov(d: f32, p: f32, reach: f32) -> f32
```

The ink drop's coverage at distance `d` - `fx_lens.wgsl::lens_ink`'s geometry, easing out on `clickfx::ease_out` and fading as it goes. 0 for a negative `p` (no drop live).

## Source

```rust
struct Source { x0: i32, y0: i32, w: i32, h: i32, px: Vec<u8> }
```

The un-shadowed, un-drawn frame under a shape's box, copied aside before anything in it changes - the source `magnify` re-samples. `(x0, y0, w, h)` in whole pixels, clipped to the frame, `px` the BGRA rows.

## snapshot

```rust
fn snapshot(out: &[u8], ow: u32, oh: u32, b: [f32; 4]) -> Source
```

Copies the box `b` (the same padded box `for_box` will visit, so every pixel the draw can touch has its original under it in the copy) out of `out`, row by row, clipped to the frame like `for_box` clips.

## magnify

```rust
fn magnify(src: &Source, c: [f32; 2], px: f32, py: f32) -> [f32; 3]
```

The magnified, lifted colour (RGB) the glass shows at `(px, py)`: the snapshot sampled bilinearly at the point `1/ZOOM` of the way out from the shape's centre `c` - a pixel shows what lies closer to the centre than itself, which is what makes everything inside uniformly `ZOOM` bigger - then multiplied by `LIFT`. Taps clamp to the snapshot's edge, and an empty snapshot answers black rather than indexing past its end.

## draw_back

```rust
fn draw_back(out: &mut [u8], ow: u32, oh: u32, b: &BackLens, accent: [u8; 3])
```

The cursor back: snapshot the box, then per pixel the shadow (outside the shape), the magnified frame blended in by the shape's coverage, then its click look - a ring hugging the pill over text, an ink drop otherwise.

## draw_glass

```rust
fn draw_glass(out: &mut [u8], ow: u32, oh: u32, l: &CursorLens, accent: [u8; 3])
```

The sprite lens: the pack's own silhouette (`LensMask`), shadowed and magnified about the box centre. The busy rotation is undone the same way `lens_cov` does it in the shader, so a spinning glass cursor's lens spins with it. The five-tap shadow matches the shader's tap pattern rather than approximating it, which is what keeps the two within the parity tests' tolerance.

## draw_lens

```rust
pub fn draw_lens(out: &mut [u8], ow: u32, oh: u32, l: &Lenses, accent: [u8; 3])
```

Both glass shapes, back first - the stacking order `fx_lens.wgsl::lens_fx` uses. `accent` is the click-FX colour, the same `u.color` the shader reads.

### Behaviors worth knowing (`fx_lensdraw_tests.rs`)

- `the_back_tints_inside_and_shadows_below` - a disc over flat grey lifts what is under it (the blue channel most, `LIFT[2]`), drops a shadow under its rim, and touches nothing outside.
- `the_back_magnifies_what_is_under_it_about_its_centre` - a 4 px vertical stripe under the disc's centre comes out `ZOOM` wider: the pixel just past the stripe's edge, background before the lens, shows the stripe inside the disc and not further out.
- `a_live_ink_drop_paints_the_accent_inside_the_back`, `the_glass_lens_only_touches_pixels_the_sprite_alpha_covers`, `an_empty_lens_pair_leaves_the_frame_untouched`.
