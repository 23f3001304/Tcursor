# src-tauri/src/export/cursor/cursorset.rs

Manages the per-type cursor sprite set for Enhanced export: decodes each cursor shape once at prep time (from the compile-time embedded built-in PNGs, or an imported pack's files via `pack::sprite_sources` - see `export/cursor/pack.rs`), looks up the active type per frame via the cursor type track, inverts RGB channels for dark themes so a single asset set serves both light and dark backgrounds, and drives the per-frame draw call. The drawing primitives live in `cursordraw`; this file is the sprite-management and call-site seam.

## SPRITES

```rust
const SPRITES: &[(CursorType, &[u8], (f32, f32))]
```

Compile-time table mapping each supported `CursorType` to its embedded PNG bytes and canvas-fraction hotspot `(x, y)`. Each row is `(type, include_bytes!(...), (hx, hy))`. Currently includes Arrow, Hand, IBeam, ResizeNs, ResizeEw, ResizeNwse, ResizeNesw, Move, and Busy. Adding a new cursor shape requires one new row here and a PNG asset in `assets/cursors/`. This is the built-in pack (`pack::DEFAULT_PACK_ID`) and also the per-kind fallback `pack::sprite_sources` uses for any kind an imported pack doesn't provide.

- *Why `include_bytes!`:* embeds all cursor PNGs in the compiled binary so the export path has no runtime file I/O dependency for the built-in sprites.
- *Why hotspot as canvas fraction:* authors define hotspots relative to the full canvas (before content-cropping); `ffio::decode_cursor` converts these to content-region fractions.
- *Why `pub(crate)` rather than private:* `pack.rs` (`sprite_sources`) and `pack_import.rs` (`collect_valid_sprites`) both iterate it - for the built-in fallback rows and to know every recognized cursor kind + its expected filename.

## CursorPrep

```rust
pub struct CursorPrep {
    pub set: HashMap<CursorType, CursorSprite>,
    pub track: CursorTrack,
    pub click_ms: Vec<u32>,
    pub recent: VecDeque<(f32, f32)>,
}
```

All per-export Enhanced cursor state, created once by `prep` and mutated each frame by `draw`.

- `set` - maps each successfully decoded `CursorType` to its `CursorSprite`. *Why HashMap:* O(1) per-frame type lookup; types that fail to decode are simply absent (non-Arrow misses fall back to Arrow).
- `track` - the per-frame cursor type sequence from the recording. *Why stored here:* `sprite_for` reads it alongside `set`; keeping them in the same struct allows the exporter to borrow both through `CursorPrep`.
- `click_ms` - sorted ascending list of `MouseDown` timestamps. *Why pre-extracted:* `cursordraw::bounce_scale` scans this list every frame; filtering the full event log per-frame would add O(n) cost per frame.
- `recent` - motion trail deque of recent hotspot positions in output pixels. *Why mutable:* `cursordraw::apply_enhanced` pushes the current position and pops old ones each frame.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - creates one `CursorPrep` per export via `prep`, then calls `draw` once per rendered frame.

Two more fields carry pack format v2's animated busy state, both resolved once at `prep` time:

- `busy: Option<BusySpec>` - the selected pack's busy animation, `None` for a still one.
- `busy_frames: Vec<CursorSprite>` - decoded `busy_NN.png` frames when the pack ships them, empty otherwise. Indexed by `BusyPose::frame`, so an out-of-range index simply falls back to `set`'s busy sprite. Decoded here, not per frame: an explicit-frame pack is a handful of extra PNG decodes at renderer-build time and zero work afterwards.

### CursorPrep::glass

```rust
pub glass: bool,
pub masks: HashMap<CursorType, std::sync::Arc<crate::export::fx::fx_lens::LensMask>>,
pub drags: Vec<crate::export::fx::fx_lens::DragSpan>,
```

The three fields the **glass cursor material** needs, all resolved once here for the same reason the sprites are: they are cheap at renderer-build time and would be per-frame work otherwise.

- `glass` - this pack declares `material: "glass"`. Resolved once because `pack::is_glass` reads `pack.json` off disk and `composite_at` asks every frame.
- `masks` - each kind's silhouette as a lens mask (`fx_lens::mask_of`). Empty unless `glass`; nine buffers of at most 16 KB.
- `drags` - every left-button press paired with its release, for the cursor back's text-selection stretch. Built even for a plain pack, since the back is pack-independent; the alternative is re-scanning the whole event log per frame.

`draw` takes its glass branch on `glass`, handing off to `cursormorph::draw_glass` (which cross-fades two states and blits at `fx_lens::SPRITE_ALPHA`) instead of `cursordraw::apply_enhanced`. Plain packs are byte-for-byte unchanged.

### CursorPrep::masks

See `CursorPrep::glass`.

### CursorPrep::drags

See `CursorPrep::glass`.

## draws_synthetic

```rust
pub fn draws_synthetic(cursor: &CursorSettings, os_cursor_in_video: bool) -> bool
```

Whether a synthetic cursor is drawn at all for these settings: `Enhanced` always, `System` only as the plain-OS stand-in for a video with no baked cursor (`CursorSettings::plain_os`), `Hidden` never.

*Why it is split out of `prep`:* `prep`'s remaining work decodes PNGs through `ffio`, which shells out to ffmpeg - so the gate itself would only be testable on a machine with ffmpeg installed. As a pure predicate it is covered exhaustively (`draws_synthetic_covers_every_style_and_bake_combination`) with no environment dependency.

*Why `os_cursor_in_video` is a parameter rather than read here:* it is RECORD-time truth, derived from the immutable `settings.json` snapshot by `settings::store::os_cursor_in_video`. It must never be inferred from `cursor.style`, which is the editable doc's value and is exactly what the user changes to trigger this path.

## prep

```rust
pub fn prep(cursor: &CursorSettings, events: &[MouseEvent], track: CursorTrack, dark: bool,
            os_cursor_in_video: bool) -> Option<CursorPrep>
```

Decodes all cursor sprites and assembles the `CursorPrep` for an export run. Sprite bytes come from `cursor.pack` via `pack::sprite_sources` (see `export/cursor/pack.rs`) - the built-in set for `pack == "default"`, or an imported pack folder falling back to the built-in sprite for any kind it doesn't provide.

### Inputs

- `cursor: &CursorSettings` - the user's cursor settings; the `draws_synthetic` gate is checked first, then `cursor.pack` selects the sprite source. *Why:* `Hidden` (and `System` on a video that already has the OS cursor baked in) returns `None` immediately, skipping all decode work.
- `os_cursor_in_video: bool` - whether the recorded video already contains a baked OS cursor. *Why:* it is what distinguishes the two meanings of `System` - "the cursor is already in the pixels, draw nothing" from "nothing is in the pixels, re-create it from the recorded path".
- `events: &[MouseEvent]` - the full mouse event log. *Why:* used only to extract `Down` timestamps into `click_ms` for the bounce animation.
- `track: CursorTrack` - the per-frame cursor type sequence from the recorder. *Why:* stored in `CursorPrep` for per-frame sprite lookup.
- `dark: bool` - whether the user's theme is dark. *Why:* cursor sprites are authored for a light background; on a dark background `invert_rgb` is applied to each decoded sprite so the cursor remains visible.

### Returns

`Some(CursorPrep)` when `draws_synthetic` passes and the Arrow sprite decodes successfully. `None` in all other cases. Non-Arrow decode failures are silently skipped (the Arrow sprite covers them as a fallback); an Arrow failure means no cursor can be drawn at all.

### Implementation

1. Return `None` immediately if `!draws_synthetic(cursor, os_cursor_in_video)`.
2. For each `(kind, png, hot)` row in `pack::sprite_sources(&cursor.pack)`, call `decode_sprite(&png, hot)`. On success, if `dark`, call `invert_rgb` on `spr.bgra` in place, then insert into `set`.
3. Verify `set.get(&CursorType::Arrow)` is `Some`; if not, return `None` (the universal fallback is required).
4. Extract `click_ms` from `events` filtered to `EventKind::Down`.
5. Return `Some(CursorPrep { set, track, click_ms, recent: VecDeque::new() })`.

### Behaviors

- `prep_is_none_for_system_and_hidden_when_the_video_has_the_os_cursor` - returns `None` for `System` (with a baked cursor) and for `Hidden` (either way) without attempting any decode.
- `draws_synthetic_covers_every_style_and_bake_combination` - all six style x baked pairs. The row that changed is `System` + no baked cursor, which now draws instead of rendering nothing at all.

## posed

```rust
fn posed<'a>(set: &'a HashMap<CursorType, CursorSprite>, track: &CursorTrack,
             busy: Option<BusySpec>, frames: &'a [CursorSprite], ev_t: u32, out_t: u32)
             -> (Option<&'a CursorSprite>, BusyPose)
```

The sprite and transform for this frame: normally the type track's own sprite, still. For the **Busy** type on a v2 pack it is `busy_pose`'s answer - an explicit frame when the pack ships them (drawn untransformed), otherwise the single busy sprite plus a rotation or scale.

*Why OUTPUT time and not `ev_t`:* the animation belongs to the rendered timeline, so one instant always yields one pose. Deterministic per exported frame, and a paused preview shows exactly the frame for where the playhead sits rather than something that depends on how the user scrubbed there. The SPRITE choice still comes from `ev_t`, because the cursor-type track is a raw event stream.

*Why it takes the prep's fields separately* (like `sprite_for`, and for the same reason): the caller still needs `&mut cp.recent` for the motion trail while holding this sprite, which only works as disjoint field borrows.


## invert_rgb

```rust
fn invert_rgb(bgra: &mut [u8])
```

Inverts the R, G, and B channels of each pixel in a BGRA buffer in place (255 - channel), leaving the alpha channel untouched.

### Inputs

- `bgra: &mut [u8]` - BGRA pixel data from a decoded `CursorSprite`. *Why:* dark-theme cursors need black/dark pixels flipped to white/bright so they remain visible against a dark background without a separate dark-theme asset pack.

### Returns

`()`. Modifies `bgra` in place.

### Behaviors

- `invert_rgb_black_to_white_keeps_alpha` - `[0, 0, 0, 255]` becomes `[255, 255, 255, 255]`.

## sprite_for

```rust
pub fn sprite_for<'a>(set: &'a HashMap<CursorType, CursorSprite>, track: &CursorTrack, ev_t: u32) -> Option<&'a CursorSprite>
```

Returns the sprite for the cursor type active at `ev_t`, falling back to Arrow if the specific type is absent from `set`.

### Inputs

- `set: &'a HashMap<CursorType, CursorSprite>` - the decoded sprite set. *Why passed separately from `CursorPrep`:* the caller (`draw`) holds a mutable borrow on `prep.recent` at the same time; splitting the fields enables a disjoint-borrow pattern so the borrow checker accepts both borrows simultaneously.
- `track: &CursorTrack` - the cursor type sequence. *Why reference, not part of the call:* same disjoint-borrow reason as above.
- `ev_t: u32` - current frame event-time. *Why:* forwarded to `track.type_at` to determine which `CursorType` was active at this moment in the recording.

### Returns

`Some(&CursorSprite)` for the active type if present, or for `CursorType::Arrow` if not. `None` only when Arrow is also absent - this cannot happen in practice because `prep` verifies Arrow before returning `Some`.

### Behaviors

- `sprite_for_empty_track_falls_back_to_arrow` - on an empty `CursorTrack`, `type_at` returns Arrow, and the Arrow sprite is found in `set`.

## draw

```rust
pub fn draw(cp: &mut CursorPrep, out: &mut [u8], ow: u32, oh: u32, cur: FramePoint,
            cam: Camera, screen: &Panel, inset_w: f32, ev_t: u32, out_t: u32, c: &CursorSettings,
            os_cursor_in_video: bool, tilt_deg: f32)
```

Per-frame synthetic cursor draw, clipped to the screen panel.

**Plain-OS mode.** When `c.plain_os(os_cursor_in_video)` - i.e. the doc asks for `System` but the video has no baked cursor - the draw is stripped back to what a real OS cursor looks like: the **Arrow** sprite regardless of the recorded type track, no click bounce, no motion trail. (The other half of the mode, the raw un-smoothed path, is applied upstream by `CursorSettings::smoothness_at`/`idealize_at` when the `Cursor` is built or reloaded.)

*Why Arrow rather than the recorded type track:* the track may not exist at all - a `Hidden` recording never ran the type tracker - and "System" promises a plain pointer, not Enhanced-minus-polish. Always-Arrow makes the fallback look the same whatever the recording style was.

*Why the mode is decided here per frame instead of being cached on `CursorPrep`:* `prep` only runs on a full renderer build, while `reload_edit` refreshes doc settings in place on every edit. Deciding at draw time means flipping the style picker updates the warm preview immediately rather than after the next rebuild.

### Inputs

- `cp: &mut CursorPrep` - the cursor prep state; `cp.recent` is updated each frame. *Why mutable:* the trail deque must be advanced.
- `out: &mut [u8]` - the BGRA frame buffer to draw into.
- `ow`, `oh` - output frame dimensions.
- `cur: FramePoint` - cursor position in base/output coordinates (before camera projection). *Why base coords:* the exporter queries `Cursor::at` in base frame space; `draw` then projects through `cam` to get the position in the final rendered output.
- `cam: Camera` - the current zoom/pan camera transform. *Why:* the cursor must be drawn at the camera-projected position in the output frame, not at the raw recording position.
- `screen: &Panel` - the screen panel's alpha and screen-space rect. *Why:* two uses - `screen.alpha < 0.5` is the early-out guard (no cursor when the screen panel is invisible), and `screen.rect` is projected into the clip box that confines the cursor to the panel bounds.
- `inset_w: f32` - width of the full inset region in output pixels. *Why:* the cursor size scales by `screen.rect.w / inset_w` so a small PiP screen gets a proportionally smaller cursor.
- `ev_t: u32` - current frame event-time; forwarded to `posed` (for the sprite) and `apply_enhanced` (for the click bounce).
- `out_t: u32` - current frame OUTPUT time, the clock pack v2's busy animation runs on. Plain-OS mode ignores it: a real OS cursor has no synthesised animation either.
- `c: &CursorSettings` - the LIVE cursor display settings (size, motion blur, click bounce, bounce intensity, and the `style` that plain-OS mode keys off).
- `os_cursor_in_video: bool` - record-time truth, threaded from `FrameRenderer`. *Why here as well as in `prep`:* `prep` decides whether there is anything to draw; this decides how to draw it, and only this one is re-evaluated per frame.
- `tilt_deg: f32` - this frame's motion lean (`Cursor::tilt_deg`, already 0 when the setting is off), **added** to whatever rotation the pose already carries. *Why composed rather than a second transform:* `blit_transformed` rotates about the hotspot exactly once, so a busy ring spinning while the cursor is thrown across the screen does both at the same time and at the same anchor - two passes would resample the sprite twice and drift. Forced to 0 in plain-OS mode alongside the bounce and the trail (belt and braces: `CursorSettings::tilt_at` has already switched the filter off upstream).

### Returns

`()`. No-op when `screen.alpha < 0.5`.

### Implementation

1. Call `frame_placement` for the position, panel factor and clip box; return immediately on `None` (screen panel more than half faded).
2. A glass pack takes the `cursormorph::draw_glass` path with `morph_at`'s busy angle **plus** `tilt_deg`, so the placed box, the cross-fade and the lens mask all lean together.
3. Otherwise call `posed(..)` for the sprite AND its busy pose - or force Arrow with a still pose in plain-OS mode - add `tilt_deg` to that pose's `angle_deg`, and on `Some(spr)` call `cursordraw::apply_enhanced` with trail cap 6, the pose, and all cursor settings from `c`. A non-zero lean makes the pose non-identity, so the draw takes `cursorxform`'s bilinear path; an upright cursor still takes the nearest-neighbour fast path exactly as before.

### Behaviors

- `a_tilted_cursor_is_the_same_sprite_rotated_about_its_hotspot` - the untilted ink box is exactly the sprite's placed box; a 6-degree lean widens and shortens it to the rotated rectangle's own extents (within the bilinear sampler's one-pixel skirt), and both boxes stay centred on the cursor point, so a leaning cursor does not drift off what it is pointing at.

## frame_placement

```rust
pub fn frame_placement(cur: FramePoint, cam: Camera, ow: u32, oh: u32, screen: &Panel,
                       inset_w: f32) -> Option<((f32, f32), f32, (i32, i32, i32, i32))>
```

Where a cursor goes this frame, whichever cursor it is. Extracted from `draw` so the captured-layer path (`export::cursor::captured::CapturedCursors::draw`) projects through the exact same math instead of a second copy of it - the two must agree on scale and clipping or a style switch would visibly move the cursor.

### Inputs

- `cur: FramePoint` - the cursor in base/output coordinates, before camera projection.
- `cam: Camera` - the frame's zoom/pan transform.
- `ow`, `oh` - output frame dimensions.
- `screen: &Panel` - the screen panel: `alpha` is the early-out gate, `rect` is both the size reference and the clip source.
- `inset_w: f32` - the full inset region's width in output pixels, the export's fixed cursor-scale reference.

### Returns

`Some((pos, panel, clip))`:

- `pos` - the hotspot's on-screen point, `cur` projected through `cam` (`coordmap::project`).
- `panel` - `(screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0)`, so a shrunk custom-arrangement screen panel shrinks the cursor with it. A LAYOUT factor, never a zoom factor - the cursor does not grow when the camera zooms in.
- `clip` - `screen.rect` projected into output pixels and clamped to the frame, as `(x0, y0, x1, y1)`. Left UNNORMALIZED: when the panel falls entirely outside the crop `x0` can exceed `x1`, and `blit`'s own `ox_start >= ox_end` check is what makes that a no-op.

`None` when `screen.alpha < 0.5` - no screen panel, no cursor.

**Dark-theme invert is gated on `pack::theme_inverts`** (2026-09-13): only the embedded default set is flipped; a bundled or imported pack keeps its own colours.

