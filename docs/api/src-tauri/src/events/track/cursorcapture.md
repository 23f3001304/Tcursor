# src-tauri/src/events/track/cursorcapture.rs

The one Win32 seam of the cursor layer: turn a live `HCURSOR` into the actual bitmap Windows is drawing it from. Windows-only - `events/track/mod.rs` gates the whole module behind `#[cfg(windows)]`, exactly as `cursortracker`'s `imp` module is gated.

Nothing in this file decides what a pixel means; it reads bitmaps and hands the bytes to `cursorpixels` (pure, unit-tested). The split exists because Win32 bitmap reads cannot be tested without a session, while the mask/BGRA rules absolutely can.

## capture

```rust
pub fn capture(cursor: HCURSOR) -> Option<CapturedCursor>
```

The bitmap `cursor` is currently drawn from, with its hotspot.

### Inputs

- `cursor: HCURSOR` - a live cursor handle, straight from `GetCursorInfo` in the tracker's poll loop. Not owned or freed by this call; the GDI bitmaps `GetIconInfo` hands back from it ARE, and are always released.

### Returns

`Some(CapturedCursor)`, or `None` when Windows will not describe the cursor (`GetIconInfo` fails for some protected/remote-session cursors), the screen DC cannot be borrowed, or the bitmaps are unreadable. The caller treats `None` as "no bitmap for this handle" and carries on with the shape track alone - a failure never ends a recording.

### Implementation

1. `GetIconInfo(HICON(cursor.0), ..)` for the hotspot (`xHotspot`/`yHotspot`) and the two bitmaps.
2. `with_dc` borrows the screen DC (`GetDC(NULL)`) for the reads and always gives it back, whatever `read` returned.
3. `read` picks the branch: `hbmColor` NULL -> monochrome (read the double-height `hbmMask`, hand it to `mono_rgba` with half its height); otherwise colour (read `hbmColor`, plus `hbmMask` for the no-real-alpha fallback, and hand both to `color_rgba`). An unreadable mask on the colour branch is not fatal - `color_rgba` then leaves the sprite opaque rather than fully transparent.
4. `describe` is `GetObjectW` into a `BITMAP`, for the size; `dib32` is `GetDIBits` at 32bpp `BI_RGB` with a **negative** `biHeight`, which is what asks GDI for top-down rows instead of a DIB's default bottom-up order.
5. `DeleteObject` on both `hbmMask` and `hbmColor` before returning, on every path. *Why this matters here specifically:* the tracker polls ~60 times a second, so a leaked pair per call would exhaust the process's GDI object quota within minutes of recording.

### Known simplifications

- **Animated cursors (`.ani`) are not animated.** `GetIconInfo` reports the frame showing at that instant, and only the first frame seen for a given handle is kept, so a spinning "busy" cursor composites as a still. Stepping frames would need `DrawIconEx` against an off-screen DC, which this file deliberately does not use.
- **The cursor is captured at its record-time pixel size.** There is no DPI renormalization; the bitmap is whatever Windows was drawing.

### Used by

- `src-tauri/src/events/track/cursortracker.rs` - calls it once per distinct `HCURSOR` seen during a take
