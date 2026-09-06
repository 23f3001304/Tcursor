# src-tauri/src/session/record/frame_fit.rs

The pure geometry of the OBS-style fit: where a capture frame of one size lands inside the encoder's fixed output canvas. Deliberately free of D3D11 and of every Windows type, so it is the part of `frame_scaler.rs` that can actually be tested — the video processor that performs the fit needs a live GPU capture and cannot be.

## letterbox

```rust
pub fn letterbox(src: (u32, u32), dst: (u32, u32)) -> (i32, i32, i32, i32)
```

The destination rect `(left, top, right, bottom)` for a `src`-sized frame scaled into a `dst`-sized canvas: aspect preserved and centred, so whatever is left over becomes the black bars — top/bottom for a relatively wider source, left/right for a taller one.

### Implementation

1. Clamp both sizes to at least `1x1`, so a degenerate input cannot divide by zero or produce an inverted rect.
2. Compare the two aspect ratios by cross-multiplying (`sw * dh >= dw * sh`) rather than dividing. *Why:* `sw/sh` against `dw/dh` in floats rounds a one-pixel-off aspect into a distortion, or a real distortion into an exact fit — and the result feeds a `RECT`, which is integers anyway.
3. The relatively wider source takes the canvas' full width and gets its height scaled (`sh * dw / sw`); the taller one takes the full height and gets its width scaled. Both are clamped into the canvas so integer rounding can never push the rect over an edge, and to a minimum of 1 so an extreme aspect still blits something rather than an empty rect.
4. Centre the result: the leftover on each axis is halved, which for an odd leftover leaves the two bars one pixel apart.

*Why scale at all, rather than crop or pad 1:1:* the fit runs for the whole rest of the take once a capture has resized. Cropping would silently cut content off the recording, and padding would shrink the picture into a corner of the frame; scaling keeps everything visible at the right proportions, which is what OBS does with a source that does not match the canvas.

### Behaviors

- `equal_sizes_fill_the_canvas_exactly`: the overwhelmingly common case — the capture never resized, so the fit is the identity and the picture is not resampled.
- `a_grown_frame_scales_down_to_fit`: `2560x1440` and `3840x2160` land as the full canvas, not a crop.
- `a_shrunk_frame_scales_up_to_fit`: `960x540` fills a `1920x1080` canvas rather than sitting small on black.
- `a_wider_frame_gets_bars_top_and_bottom`: `1920x1000` into `1920x1080` gives `(0, 40, 1920, 1040)`.
- `a_taller_frame_gets_bars_left_and_right`: `1920x1200` into `1920x1080` gives `(96, 0, 1824, 1080)`.
- `the_bookmarks_bar_case_keeps_the_full_width`: the bug this whole module exists for — switching Chrome tabs toggles the bookmarks bar, so the capture loses ~30 rows mid-take. `1600x870` into `1600x900` keeps the full width with a thin band, instead of the picture freezing on the last full-height frame.
- `the_fit_is_centred_and_never_distorts`: over a spread of shapes (including a 3440x1440 ultrawide and a 17x991 sliver), the rect stays inside the canvas, touches one pair of its edges, is centred to within a pixel, and matches the ideal uniformly scaled rect to within a pixel on both axes — i.e. one scale factor for both, never two.
