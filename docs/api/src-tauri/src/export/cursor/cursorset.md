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
            cam: Camera, screen: &Panel, inset_w: f32, ev_t: u32, c: &CursorSettings,
            os_cursor_in_video: bool)
```

Per-frame synthetic cursor draw, clipped to the screen panel.

**Plain-OS mode.** When `c.plain_os(os_cursor_in_video)` - i.e. the doc asks for `System` but the video has no baked cursor - the draw is stripped back to what a real OS cursor looks like: the **Arrow** sprite regardless of the recorded type track, no click bounce, no motion trail. (The other half of the mode, the raw un-smoothed path, is applied upstream by `CursorSettings::follow_alpha_at`/`idealize_at` when the `Cursor` is built or reloaded.)

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
- `ev_t: u32` - current frame event-time; forwarded to `sprite_for` and `apply_enhanced`.
- `c: &CursorSettings` - the LIVE cursor display settings (size, motion blur, click bounce, bounce intensity, and the `style` that plain-OS mode keys off).
- `os_cursor_in_video: bool` - record-time truth, threaded from `FrameRenderer`. *Why here as well as in `prep`:* `prep` decides whether there is anything to draw; this decides how to draw it, and only this one is re-evaluated per frame.

### Returns

`()`. No-op when `screen.alpha < 0.5`.

### Implementation

1. Return immediately if `screen.alpha < 0.5`.
2. Project `cur` through `cam` via `coordmap::project` to get the output-space hotspot `pos`.
3. Compute `panel = (screen.rect.w / inset_w.max(1.0)).clamp(0.1, 1.0)` as the cursor size scale.
4. Project `screen.rect` via `project_rect` to get the `clip` box `(x0, y0, x1, y1)` in output pixels.
5. Call `sprite_for(&cp.set, &cp.track, ev_t)`. On `Some(spr)`, call `cursordraw::apply_enhanced` with trail cap 6 and all cursor settings from `c`.
