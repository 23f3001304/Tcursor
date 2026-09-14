# src-tauri/src/export/fx/fx_lensbuild.rs

Where the glass shapes GO this frame. Split from `fx_lens.rs` (the types and the curves) so both stay inside the size limit, and kept out of `cursorset.rs` for a reason that is not size: the FX pass needs the answer BEFORE the cursor blit runs.

**The order, and why the placement lives here.** `render::composite_at` composites the frame, asks this file where the glass is, hands that to `fx_state::render` (which refracts through it), and only THEN blits the cursor sprite on top. The lens is therefore computed one step before the thing it belongs to is drawn. Putting it in `cursorset` would mean the cursor pass computing a box, throwing it away, and the FX pass - which already ran - recomputing it.

## LensFrame

```rust
#[derive(Clone, Copy)]
pub struct LensFrame<'a> {
    pub cur: FramePoint, pub cam: Camera, pub ow: u32, pub oh: u32,
    pub screen: &'a Panel, pub inset_w: f32,
    pub info: &'a ScreenInfo, pub src: RectF,
    pub ev_t: u32, pub out_t: u32,
    pub tilt_deg: f32,
}
```

Everything the placement needs about the frame the cursor is being drawn into, bundled so `lenses_at` takes three arguments instead of thirteen.

- `cur`/`cam`/`ow`/`oh`/`screen`/`inset_w` - exactly what `cursorset::frame_placement` takes, so the lens and the sprite are placed by the same call.
- `info`/`sw`/`sh` - the recording's `ScreenInfo` and source size, used only to convert the text-selection anchor from a raw desktop point (see `desktop_to_out`).
- `ev_t`/`out_t` - the two clocks. The cursor-kind track and the mouse stream are event-time; the busy animation is output-time, so a paused preview shows one pose per instant.
- `tilt_deg` - this frame's motion lean (`Cursor::tilt_deg`). *Why the lens needs it:* the lens IS the cursor's own silhouette, so it has to tip with the sprite; a leaning glass cursor over an upright lens would bend the frame in one direction while the glass pointed in another. `lens_of` adds it to `morph_at`'s busy angle, the same sum `cursorset::draw` hands `cursormorph::draw_glass` - the glass and the frame it bends stay in step because both read this one number.

`src` is the canvas sub-rect the screen panel is showing this frame (`Scene.src`) - the whole canvas normally, one display switch's fitted rect after a mid-take switch. It replaced the bare `sw`/`sh` pair so `desktop_to_out` maps through exactly the rect `fx_state_at` maps a click through.

## lenses_at

```rust
pub fn lenses_at(cp: &CursorPrep, c: &CursorSettings, f: LensFrame, plain_os: bool) -> Option<Lenses>
```

The glass lens and/or cursor back for one frame. `None` when neither applies: the pack is not a glass one and the back is off, the screen panel is more than half faded (no screen, no cursor), or this is the plain-OS cursor.

**Plain-OS gets nothing.** `System` on a recording that baked no cursor promises a plain arrow, not "Enhanced minus polish" - the same rule `cursorset::draw` already applies to the bounce and the trail, and `stageCursor.ts` mirrors on the preview side.

**Squash, and why the sprite lens usually has none.** The sprite lens's box already carries `cursordraw::bounce_scale`'s click dip, so applying `fx_lens::squash_at` on top of it would dip twice. Its own squash therefore only applies when `click_bounce` is off, which is what keeps the glass reacting to a click for a user who turned the bounce off. The back has no sprite to follow, so it always squashes.

### Used by

- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::lenses`, once per composited frame.

## lens_of

```rust
fn lens_of(cp: &CursorPrep, kind: CursorType, prev: CursorType, m: f32, centre: [f32; 2],
           size: [f32; 2], f: LensFrame, squash: f32, ink: f32, pos: (f32, f32)) -> Option<CursorLens>
```

The sprite lens itself: the already-interpolated box, the rotation in radians, and the mask - the settled kind's `Arc` clone, or both kinds' blended (`fx_lens::morph_mask`) while a state change is still easing.

The rotation is `morph_at`'s busy angle **plus** `f.tilt_deg`, converted to radians once here. Both are rotations about the same hotspot, so they add rather than compose: a busy ring spinning while the cursor is thrown across the screen bends the frame by the sum, exactly as the sprite over it is drawn by the sum.

## morph_at

```rust
pub fn morph_at(cp: &CursorPrep, ev_t: u32, out_t: u32) -> (CursorType, CursorType, f32, f32)
```

The cursor state change in flight at `ev_t`: `(previous kind, current kind, eased progress, busy rotation in degrees)`.

THE one answer both the lens (`lens_of`) and the sprite cross-fade (`cursormorph::draw_glass`) are built from, so the glass and the frame it bends cannot disagree about which shapes are dissolving or how far along they are.

The rotation is scaled by the morph in whichever direction it runs - a ring spins up as it arrives and unwinds as it leaves - and is 0 for a pack shipping explicit busy frames, which has no synthesised rotation at all.

### Used by

- `src-tauri/src/export/cursor/cursorset.rs` - `draw`, on the glass-pack branch.

## desktop_to_out

```rust
fn desktop_to_out(p: [i32; 2], f: LensFrame) -> [f32; 2]
```

A raw `WH_MOUSE_LL` desktop point in OUTPUT pixels - the same three-step conversion every click hit takes in `fx_state_at` (desktop to source frame, source frame to screen panel, then the camera projection), so the selection bar's anchor lands exactly where that click's ripple does.

## wants_lens

```rust
pub fn wants_lens(cp: Option<&CursorPrep>, c: &CursorSettings, plain_os: bool) -> bool
```

Whether a glass shape is live at all this frame - the cheap check `render::composite_at` makes before bothering to build one. Reads `CursorPrep::glass` rather than `pack::is_glass`, because the latter reads `pack.json` off disk and this runs per frame.
