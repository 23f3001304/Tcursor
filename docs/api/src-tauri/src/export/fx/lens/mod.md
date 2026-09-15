# src-tauri/src/export/fx/lens/mod.rs

The **glass cursor material**: the shapes and the curves behind a refracting cursor. Two independent things live under that name and this file describes both.

1. **The sprite lens.** A cursor pack whose `pack.json` declares `material: "glass"` (`pack::is_glass`) does not have its sprites blitted as pictures. The FX pass bends the recorded frame through each sprite's own silhouette, and only then is the pack's artwork laid on top at `SPRITE_ALPHA`, contributing its baked highlights and rim. The sprite is the LENS, not the finished look.
2. **The cursor back.** A pack-independent setting (`settings::cursor::CursorBack`) that puts a glass shape BEHIND whatever cursor is drawn - a disc, a horizontal pill over text, a bar stretched along a selection.

**Where the pieces are.** This file is types + timing math only. `fx_lensmask.rs` builds and blends the silhouettes, `fx_lensbuild.rs` places both shapes for one frame, `fx_lens.wgsl` renders them on the GPU, `fx_lensdraw.rs` is the CPU stand-in, and `cursormorph.rs` cross-fades the sprites that land on top. The live canvas preview mirrors the placement in `src/editor/stage/cursor/cursorGlass.ts`.

**One clock, no state.** Everything here is a pure function of an event-clock timestamp plus the recorded tracks (the cursor-kind track and the mouse stream), so one instant always yields one shape. That is the same determinism `busy.rs` keeps, and it is what lets a paused editor preview show exactly the frame the export would write, however the playhead got there.

## GLASS

```rust
pub const GLASS: &str = "glass";
```

The one `pack.json` `material` value the renderer understands. `pack::is_glass` compares against it; anything else (including absent) is today's plain alpha blit.

## SPRITE_ALPHA

```rust
pub const SPRITE_ALPHA: f32 = 0.65;
```

The alpha a glass pack's own sprite is blitted at, over the refraction the FX pass already drew. *Why not 1.0:* an opaque blit hides the bent frame completely, which is the bug this whole feature exists to avoid - the pack's pixels would be a picture OF glass instead of a lens. *Why not lower:* the artwork's rim and specular highlights are what make the silhouette read as an edge; below about 0.5 they stop registering at cursor size. Mirrored by `cursorGlass.ts::GLASS_ALPHA` and pinned on both sides.

## ZOOM

```rust
pub const ZOOM: f32 = 1.35;
```

The magnification inside a glass shape: a pixel shows what lies `1/ZOOM` of its distance from the shape's centre, so everything under the glass is uniformly this much bigger - and readable. Owner ruling 2026-09-14: the glass must zoom AND stay readable. Before it, the shader only displaced the rim (nothing was magnified at the centre), frosted the whole shape with four taps 3-4 px apart, and mixed the back at 85% over the un-magnified frame - on 14 px text that read as a smear with a ghost of every letter under it. Now the zoom is uniform, the frost closes to one crisp sample at the centre (it only softens the rim), and the back fully replaces what is under it. `fx_lens.wgsl::LENS_ZOOM` and `cursorGlass.ts::LENS_ZOOM` carry the same number; `lens/draw.rs::magnify` reads this one.

## BACK_SCALE

```rust
pub const BACK_SCALE: f32 = 2.2;
```

The cursor back's diameter as a multiple of the sprite's DRAWN height, so it tracks the cursor-size setting and the screen-panel scale without a second knob.

## PILL_W

```rust
pub const PILL_W: f32 = 0.35;
```

The back's height over text as a fraction of its width: the disc flattens into a HORIZONTAL pill. *Why horizontal:* a line of text is horizontal, the selection it stretches into is horizontal, and the Crystal pack's own I-beam sprite is a horizontal pill - a vertical one read as an unrelated object sitting on the line.

## SQUASH

```rust
pub const SQUASH: f32 = 0.92;
pub const SQUASH_MS: f32 = 120.0;
```

The click squash: the glass compresses to 92% at the instant of a press and is back to 1.0 over 120 ms. Applied to the back always; applied to the SPRITE lens only when `click_bounce` is off, because the sprite box already carries `cursordraw::bounce_scale`'s dip and squashing twice doubles it (`lens::build::lenses_at`).

## SQUASH_MS

See `SQUASH`.

## INK_MS

```rust
pub const INK_MS: f32 = 260.0;
```

How long the click ink-drop takes to spread through a shape before it is gone.

## MORPH_MS

```rust
pub const MORPH_MS: f32 = 160.0;
```

How long a shape change eases over - a cursor kind change (disc to pill), and the selection bar's stretch in and retraction out. The easing itself is the click effects' `clickfx::ease_out`, deliberately shared so a morph and a ripple started at the same instant move together. Mirrored by `cursorGlass.ts::MORPH_MS`.

## CursorLens

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct CursorLens {
    pub cbox: [f32; 4], pub angle: f32,
    pub squash: f32, pub ink: f32, pub ink_at: [f32; 2],
    pub mask: Arc<LensMask>,
}
```

The sprite-shaped lens for one frame, in OUTPUT pixels.

- `cbox` - `[centre x, centre y, width, height]`, the sprite's placed box. Already interpolated through a cursor-state change (`cursormorph::morph_box`), so the refraction morphs with the glass instead of snapping under it.
- `angle` - the busy rotation in RADIANS (the shader's unit), scaled by the morph so a ring spins up as it arrives and unwinds as it leaves.
- `squash` - see `SQUASH`. 1.0 at rest.
- `ink` - the click ink-drop's progress in 0..1, or negative when no drop is live.
- `ink_at` - the drop's origin in output px: the cursor's own hotspot point. *Why not the recorded click point:* within the drop's 260 ms the cursor has moved a few pixels at most, the drop is masked to the sprite anyway, and using the cursor point saves threading the whole screen-to-output conversion chain into the builder for a difference nobody can see.
- `mask` - the silhouette the refraction is confined to (`lens::mask`). `Arc` because the settled per-kind masks are built once at prep time and shared by every frame.

## BackLens

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackLens {
    pub mn: [f32; 2], pub mx: [f32; 2], pub r: f32,
    pub squash: f32, pub ink: f32, pub ink_at: [f32; 2], pub ring: bool,
}
```

The cursor back for one frame: ONE rounded rect (`mn`/`mx` corners, `r` corner radius) which is a circle when `r` is half the height, a horizontal pill when it is not, and a selection bar when `mn.x`/`mx.x` have been stretched. *Why one primitive for three shapes:* the morph between them is then a lerp of two corners and a radius, with no per-pair special case, and the shader needs one signed-distance function rather than three.

`ring` marks the over-text look, where a click draws a thin line hugging the pill instead of an ink drop.

## Lenses

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Lenses { pub glass: Option<CursorLens>, pub back: Option<BackLens> }
```

Both glass shapes for one frame; either half may be absent. Carried on `FxState::lens` and rendered back-first, so the sprite lens lands on top of the back and the pack's own pixels land on top of both.

## DragSpan

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragSpan { pub down: u32, pub up: u32, pub x: i32, pub y: i32 }
```

One left-button press and the release that ended it (`u32::MAX` while still held), with the press's raw desktop point. Pre-extracted once at `cursorset::prep` time into `CursorPrep::drags` so the per-frame lookup is a binary search rather than a scan of the whole event log - the same reason `click_ms` exists beside it.

## squash_at

```rust
pub fn squash_at(clicks: &[u32], t: u32) -> f32
```

The lens scale at event time `t`: `SQUASH` at the instant of the most recent click at or before `t`, easing linearly back to 1.0 over `SQUASH_MS`. 1.0 when no click has happened yet.

## ink_at

```rust
pub fn ink_at(clicks: &[u32], t: u32) -> f32
```

The ink drop's progress 0..1 at event time `t`, or a NEGATIVE number when no drop is live. *Why negative rather than `Option`:* the value is packed into a shader uniform slot, where "no drop" has to be a float anyway - one representation for both sides beats converting at the boundary.

## drag_spans

```rust
pub fn drag_spans(events: &[MouseEvent]) -> Vec<DragSpan>
```

Every left-button press in the event log paired with its release. A press with no release yet gets `u32::MAX`, which makes `selection_at`'s "still held" test a plain `s.up > t`.

## selection_at

```rust
pub fn selection_at(spans: &[DragSpan], t: u32) -> Option<([i32; 2], f32)>
```

The text-selection anchor at event time `t`: the DESKTOP point the left button went down at, and how far the stretch has eased in. 1.0 while held, easing back to 0 over `MORPH_MS` after the release, `None` once that is over - so the bar RETRACTS to the cursor instead of vanishing off the line.

The returned point is raw `WH_MOUSE_LL` desktop coordinates; `lens::build::desktop_to_out` puts it through the same three-step conversion every click hit takes, so the bar's far end lands exactly where that click's ripple would.

## kind_morph

```rust
pub fn kind_morph(track: &CursorTrack, t: u32) -> (CursorType, CursorType, f32)
```

The cursor state change in flight at event time `t`: `(current kind, previous kind, eased progress)`. `1.0` - settled, nothing to interpolate - for an empty track, for a time before the first sample, and for any sample older than `MORPH_MS`.

Read off the recorded track rather than remembered across frames, which is what keeps a scrubbed preview honest: the morph at one instant does not depend on which instant was rendered before it.

Mirrored by `cursorGlass.ts::cursorMorphAt`, tested against the same instants on both sides.

## back_geom

```rust
pub fn back_geom(kind: CursorType, prev: CursorType, m: f32, pos: [f32; 2], h: f32,
                 sel: Option<([f32; 2], f32)>, squash: f32, ink: f32) -> BackLens
```

The cursor back for one frame: `kind`'s shape morphed `m` of the way from `prev`'s, centred on `pos` and sized from the sprite's drawn height `h`. With `sel` present AND the kind being `IBeam`, the x extent grows to reach the press point, weighted by the ease `selection_at` returned.

`pos` is the sprite's BOX CENTRE, not the hotspot: an arrow's hotspot is its tip, so a disc centred there sits up and to the left of the cursor it is meant to be behind. Over text the two coincide, which is where the pill's alignment actually matters.

### Used by

- `src-tauri/src/export/fx/lens/build.rs` - `lenses_at`, once per frame when the back is on.
