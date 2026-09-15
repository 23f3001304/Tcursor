# src-tauri/src/export/cursor/draw/cursormorph.rs

Cross-fading one cursor STATE into the next, for a `material: "glass"` pack only.

**Why glass packs alone.** Crystal's shapes are objects, not pictures of pointers - a frosted disc for the pointer, a horizontal pill for text, an oriented capsule for a resize - so snapping from one to the next reads as a different object being swapped in rather than one thing changing. Instead the placed box is interpolated from the previous kind's to the new one's over `fx_lens::MORPH_MS`, both sprites are drawn stretched into THAT box, and their alphas cross-fade through it. Plain packs never come here: their cursors are pictures of pointers, where a snap is what an OS cursor does and a dissolve would look like a bug.

**Why the lens rides along.** `fx_lensbuild::lens_of` places the refraction on the same box and blends the same two silhouettes (`fx_lens::morph_mask`), so the bent frame morphs with the glass rather than jumping under it. Both sides read `fx_lensbuild::morph_at`, which is the single source of "which shapes, how far along".

## sprite_box

```rust
pub fn sprite_box(spr: &CursorSprite, pos: (f32, f32), size_px: f32, bounce: f32) -> [f32; 4]
```

A sprite's placed box in OUTPUT px as `[x0, y0, w, h]`: the hotspot lands on `pos`, and the scale is the canvas-relative one every cursor uses (`cursordraw::draw_cursor_posed`), so a wide resize arrow stays wide instead of being stretched to the pointer's height.

THE one definition: `fx_lensbuild` places the lens with it and `draw_glass` blits with it, so the glass and the frame it bends can never disagree about where the cursor is.

### Inputs

- `size_px: f32` - the target full-canvas height in output px, i.e. `size * oh * 0.033 * panel`.
- `bounce: f32` - `cursordraw::bounce_scale`'s click dip. Scaling here rather than at the call sites is what keeps the box centred on the hotspot through the dip.

## lerp_box

```rust
pub fn lerp_box(a: [f32; 4], b: [f32; 4], m: f32) -> [f32; 4]
```

`a` lerped to `b` by `m`, component-wise, with `m` clamped. `m` arrives already eased by `fx_lens::kind_morph` - the easing is not repeated here, so a caller that wants a linear blend can have one.

## morph_box

```rust
pub fn morph_box(prev: &CursorSprite, cur: &CursorSprite, m: f32, pos: (f32, f32),
                 size_px: f32, bounce: f32) -> [f32; 4]
```

The box the cursor occupies this frame: `cur`'s own once settled, or the interpolation from `prev`'s while a kind change is still easing. Because both boxes put their own hotspot on `pos`, every point along the interpolation does too - a morph changes shape without the cursor drifting off its own point.

## one

```rust
fn one(out: &mut [u8], ow: u32, oh: u32, spr: &CursorSprite, dest: [f32; 4], alpha: f32,
       angle_deg: f32, clip: (i32, i32, i32, i32))
```

One sprite, stretched into `dest` at `alpha`.

A non-zero `angle_deg` - a spinning busy ring, or a morph on its way into or out of one - needs `cursorxform::blit_transformed`, which only takes a UNIFORM scale; there the sprite is height-matched to `dest` and the aspect stretch is dropped. That only ever applies while a rotation is in flight, where nothing is settled enough to read an aspect off anyway.

## draw_glass

```rust
pub fn draw_glass(cp: &mut CursorPrep, out: &mut [u8], ow: u32, oh: u32, pos: (f32, f32),
                  prev_kind: CursorType, cur_kind: CursorType, m: f32, angle_deg: f32,
                  panel: f32, clip: (i32, i32, i32, i32), ev_t: u32, c: &CursorSettings,
                  alpha: f32)
```

The whole glass-pack cursor draw for one frame: motion trail, then the two states cross-faded (outgoing `1 - m`, incoming `m`, both scaled by `alpha`) inside one interpolated box.

**The trail is the current state only**, and never cross-faded: it is a fading echo of where the cursor WAS, and dissolving each ghost through a second shape reads as noise - the same reasoning `cursordraw::draw_cursor_posed` uses for never applying the busy transform to the trail.

**One limitation, stated.** A glass pack shipping EXPLICIT `busy_NN.png` frames is not honoured here: the pack's `busy.png` is drawn under the synthesised rotation instead, because the lens mask is per-kind and an explicit frame has no mask of its own. No shipped glass pack has frames.

`cp` is taken mutably for the trail ring, and the two sprites come out of `cp.set` at the same time - disjoint field borrows, the same trick `cursorset::sprite_for` exists for.

### Used by

- `src-tauri/src/export/cursor/pack/cursorset.rs` - `draw`, on the glass branch, in place of `cursordraw::apply_enhanced`.
