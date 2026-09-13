# src-tauri/src/events/track/cursorpixels.rs

Pure conversion of Win32 cursor bitmaps into straight-alpha, top-down RGBA. `cursorcapture.rs` does the GDI talking and hands the raw 32bpp DIB rows here; every pixel rule lives in this file so it can be unit-tested without a Windows session, a real cursor, or a device context.

Two shapes of cursor exist and both land here:

- **Colour** - `ICONINFO.hbmColor` is a real bitmap. Usually 32bpp with a genuine alpha channel; some are 32bpp with an all-zero alpha channel (their shape lives in the AND mask instead), and some are lower-depth bitmaps that `GetDIBits` widens to 32bpp with zero alpha. The last two cases are indistinguishable from each other and are handled identically.
- **Monochrome** - `hbmColor` is NULL and `hbmMask` is twice the cursor's height: the AND mask on top, the XOR mask below. The I-beam is the classic example.

## CapturedCursor

```rust
pub struct CapturedCursor { pub w: u32, pub h: u32, pub hx: u32, pub hy: u32, pub rgba: Vec<u8> }
```

One captured OS cursor bitmap, ready to be PNG-encoded into the project's cursor layer.

- `w`, `h` - size in captured pixels (the cursor's real on-screen size at record time).
- `hx`, `hy` - hotspot in pixels from the top-left, straight from `ICONINFO`. This is the point the OS puts on the mouse position, so it is what the export aligns with the recorded cursor path.
- `rgba` - `w * h * 4` bytes, **straight** (non-premultiplied) alpha, **top-down** rows. Straight alpha because that is what PNG stores and what `cursordraw::blit` expects.

### Used by

- `src-tauri/src/events/track/cursorcapture.rs` - produced by `capture`
- `src-tauri/src/events/track/cursorlayer.rs` - consumed by `CursorLayerBuilder::add`

## color_rgba

```rust
pub fn color_rgba(bgra: &[u8], and32: &[u8], w: u32, h: u32, top_down: bool) -> Option<Vec<u8>>
```

Convert a colour cursor's bitmap to RGBA.

### Inputs

- `bgra: &[u8]` - `hbmColor` read back as 32bpp BI_RGB, i.e. `(blue, green, red, alpha)` per pixel.
- `and32: &[u8]` - `hbmMask` read back the same way, so a mask bit of 1 arrives as white and 0 as black. *Why 32bpp rather than the mask's native 1bpp:* it removes all bit-unpacking and stride-padding arithmetic from the conversion, at the cost of one extra `GetDIBits` call per cursor (done once per distinct cursor per recording). May be empty when the mask could not be read.
- `w`, `h` - the colour bitmap's size.
- `top_down: bool` - whether row 0 of `bgra`/`and32` is the TOP scanline. A DIB is bottom-up by default; `GetDIBits` only returns top-down rows when asked with a negative `biHeight`, so the order is a property of the read and has to travel with the bytes.

### Returns

`Some(rgba)` of `w * h * 4` bytes, or `None` when `bgra` is too short for the claimed size (a short buffer is declined rather than read past).

### Implementation

1. Scan the alpha bytes. If ANY is non-zero the cursor has real alpha and it is used verbatim.
2. Otherwise, if `and32` is long enough, alpha comes from the mask: bit 1 (white) -> 0 (transparent), bit 0 -> 255. *Why:* a 32bpp cursor with an all-zero alpha channel decodes fully invisible without this.
3. If neither holds (no real alpha AND no readable mask) every pixel is opaque - a visible cursor beats a transparent one.
4. Channels are reordered BGRA -> RGBA as they are copied.

## mono_rgba

```rust
pub fn mono_rgba(mask32: &[u8], w: u32, h: u32, top_down: bool) -> Option<Vec<u8>>
```

Convert a monochrome (`hbmColor == NULL`) cursor to RGBA.

### Inputs

- `mask32: &[u8]` - the double-height `hbmMask` read as 32bpp: `h` AND rows followed by `h` XOR rows, in the BITMAP's own order (so under `top_down: false` the buffer's first row is the XOR mask's last).
- `w`, `h` - the CURSOR's size, i.e. half the mask bitmap's height.
- `top_down: bool` - as for `color_rgba`.

### Returns

`Some(rgba)`, or `None` when `mask32` is shorter than two full row blocks.

### Implementation

Per pixel, from the Win32 AND/XOR rules:

| AND | XOR | Win32 meaning | Rendered |
| --- | --- | --- | --- |
| 0 | 0 | black | opaque black |
| 0 | 1 | white | opaque white |
| 1 | 0 | leave the screen alone | transparent |
| 1 | 1 | invert the screen | **opaque white** |

The last row is a **documented simplification**: a pre-composited sprite cannot invert what is behind it, and inverted pixels exist to keep the cursor visible over dark surfaces, so opaque white is the closest still. No black outline is synthesized.

*Note on the AND=0 rows:* they are split by XOR rather than collapsed to black, which is what makes the I-beam's white outline survive - collapsing them would make the I-beam invisible on a dark editor or terminal.
