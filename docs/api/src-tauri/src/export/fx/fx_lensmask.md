# src-tauri/src/export/fx/fx_lensmask.rs

The lens **mask**: which pixels of a frame the glass covers, and how one cursor state's silhouette dissolves into the next. Split out of `fx_lens.rs` (the shapes and the curves) for the 200-line limit; both halves are `pub use`d from there, so callers keep saying `fx_lens::mask_of`.

## LensMask

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct LensMask { pub key: u64, pub w: u32, pub h: u32, pub a: Vec<u8> }
```

One cursor silhouette as a single-channel coverage buffer, row-major, `w * h` bytes.

- `key` - identifies what the buffer holds. `GpuFx` keeps ONE uploaded mask and re-uploads only when this changes, so a settled cursor pays nothing per frame and a morph pays about 16 KB per frame for the ten frames it lasts.
- `a` - coverage, not opacity. See `mask_of`.

## MASK_IN

```rust
const MASK_IN: f32 = 0.02;
const MASK_FULL: f32 = 0.10;
```

Where a sprite's alpha stops being background and starts being the lens.

*Why a threshold at all.* A clear-glass sprite is a bright opaque RIM around a body of roughly 10-40% alpha - that faint body IS the glass. Used verbatim, the alpha would refract the body at a tenth strength and the lens would be all but invisible, which is exactly what the first bench frames showed. Saturating between these two makes the whole silhouette the lens while keeping the sprite's one-pixel antialiased outer edge soft.

The sprite's own opacity is not lost: it still shows, in the 65%-alpha blit that lands on top (`fx_lens::SPRITE_ALPHA`).

## MASK_FULL

See `MASK_IN`.

## MORPH_MASK

```rust
const MORPH_MASK: u32 = 128;
```

Side of the square buffer a cross-faded mask is resampled into - the bundled packs' own sprite size, so a settled mask is never blurrier than the sprite it came from.

## mask_of

```rust
pub fn mask_of(pack: &str, kind: CursorType, spr: &CursorSprite) -> LensMask
```

One decoded sprite's silhouette, keyed by its pack and kind. Built once per kind at `cursorset::prep` time (into `CursorPrep::masks`) and only for a glass pack: nine buffers of at most 16 KB, versus re-deriving one every frame of an export.

The key hashes the pack ID and the kind together, so a cached upload can never be reused for the wrong sprite - including across two packs that happen to ship the same file.

### Inputs

- `pack: &str` - the selected pack ID, part of the cache key only.
- `kind: CursorType` - likewise.
- `spr: &CursorSprite` - the already-decoded, content-cropped, theme-corrected sprite. Cropped is what makes the mask's normalized space match the placed box's.

## morph_mask

```rust
pub fn morph_mask(prev: &LensMask, cur: &LensMask, p: f32) -> LensMask
```

The mask MID-MORPH: both kinds' silhouettes resampled into one `MORPH_MASK`-square buffer and blended by `p`.

*Why a plain lerp works.* The two sprites are drawn stretched into the SAME interpolated box (`cursormorph::draw_glass`), so their masks share that box's normalized space - no per-kind hotspot or aspect fix-up is needed. One glass state therefore dissolves into the next INCLUDING the refraction, which is the whole point: a snap of the bent frame is far more obvious than a snap of the sprite.

The returned key carries both sources and the quantized step, so the GPU's one-entry cache re-uploads through the morph and stops the moment it settles, and the two ends of a morph are never confused with each other (nor with the same pair running the other way).

### Used by

- `src-tauri/src/export/fx/fx_lensbuild.rs` - `lens_of`, only while `kind_morph` reports a change still in flight.
