# src-tauri/src/events/remap.rs

Where a point on the display a take switched TO lands on the display the take STARTED on - the input half of mid-take display switching (2026-09-14, `docs/superpowers/specs/2026-09-14-mid-take-source-switching-design.md`).

A switch (`session::record::switch_display`) restarts the WGC capture on another monitor but keeps the encoder, so every later frame is fitted into the first display's canvas by `frame_scaler` / `frame_fit::letterbox`: aspect preserved, centred, black bars where the aspects differ. The input streams get no such treatment for free. The mouse, click and typing trackers record raw `WH_MOUSE_LL` desktop coordinates, and the export turns those into frame pixels by subtracting the take's ONE `ScreenInfo` origin (`export::coordmap::to_frame`). Left alone they would describe the second monitor's desktop rectangle against the first monitor's picture, so the drawn cursor, the click ripples and every auto-zoom anchor would land somewhere else entirely - typically a whole screen width away.

So the samples are mapped where the pixels are, at capture time (`events::track::tracker`), and `events.json` keeps one screen and one `ScreenInfo`. Nothing downstream of the hook changes.

Integer arithmetic throughout, like `letterbox` itself, so a same-sized second display maps exactly - identity plus the origin shift - rather than accumulating a float rounding drift across a take.

## Remap

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Remap {
    from_origin: (i32, i32),
    from_size: (u32, u32),
    canvas_origin: (i32, i32),
    canvas_size: (u32, u32),
    fit: (i32, i32, i32, i32),
}
```

The fixed mapping installed on the mouse hook for as long as a take is capturing a display other than its own. `Copy` so `tracker`'s global `Sink` can hold it behind its mutex and the hook can read it without an allocation on the hot path.

- `from_origin` / `from_size` - the desktop origin and the size of the display now being captured, from `session::record::target_bounds::get_target_bounds`. Sizes are floored at 1 so a target that reports zero cannot divide by zero.
- `canvas_origin` / `canvas_size` - the take's own display: `Running.screen`, the single `ScreenInfo` written into `events.json`, and therefore the encoder canvas every frame is fitted into.
- `fit` - `letterbox(from_size, canvas_size)` as `(left, top, right, bottom)`: where the new display's picture actually sits inside that canvas. Computed once, at the switch, not per sample.

## Remap::for_switch

```rust
pub fn for_switch(new_origin: (i32, i32), new_size: (u32, u32), canvas_origin: (i32, i32), canvas_size: (u32, u32)) -> Remap
```

The mapping for a switch onto a display at `new_origin` sized `new_size`, for a take whose canvas is the display at `canvas_origin` sized `canvas_size`. Pure; the only work is the one `letterbox` call.

*Why the canvas is passed rather than assumed:* a take can start on a monitor that is not the primary one, so its `ScreenInfo` origin is routinely non-zero (and negative for a monitor arranged to the left of the primary). The result has to be expressed in THAT display's absolute coordinates, because `to_frame` subtracts that origin and not zero.

## Remap::apply

```rust
pub fn apply(&self, x: i32, y: i32) -> (i32, i32)
```

Maps one desktop point on the captured display to the desktop point on the take's own display that shows the same pixel:

```
(x - from_origin.x) * fit_w / from_w + fit_x + canvas_origin.x
```

and the same for y, each axis clamped to the canvas. The result stays ABSOLUTE - it is what the hook writes into `events.json`, and the export subtracts the take's origin from it afterwards exactly as it always has.

## axis

```rust
fn axis(v: i32, origin: i32, size: u32, fit: (i32, i32), canvas_origin: i32, canvas_len: u32) -> i32
```

One axis of `apply`: local offset on the captured display, scaled into the fit rect, shifted by the rect's offset and the canvas' own origin, then clamped to the canvas.

`i64` throughout because `offset * fit_len` overflows `i32` at ordinary desktop sizes (a 3440-wide offset times a 1920-wide fit is already 6.6M, and a stray hook sample far off screen multiplies much further). Integer division truncates toward zero, which is the same half-pixel slack `letterbox` itself carries and is invisible against a cursor hotspot.

*Why the clamp:* a three-monitor desktop can put the pointer on a display that is neither the take's nor the captured one, and a hook sample can also arrive with a coordinate outside every monitor. Without the clamp such a sample becomes an off-frame anchor that the auto-zoom camera would chase off the edge of the composition. Pinning it to the canvas edge keeps it in the frame the editor knows about. The clamp is to the CANVAS, not to the fit rect, so a point in a black bar stays in the black bar - that bar is part of the recorded picture.

### Behaviors

- `a_same_size_display_is_the_identity_plus_the_origin_shift`: a 1920x1080 second monitor at `(1920, 0)` into a 1920x1080 canvas at the origin maps `(1920, 0) -> (0, 0)`, `(2220, 200) -> (300, 200)`, `(3839, 1079) -> (1919, 1079)` - no scaling, no bars.
- `the_canvas_origin_is_carried_into_the_result`: with the take's own display at `(-1920, 0)`, `(300, 200)` maps to `(-1620, 200)`, i.e. still in that display's absolute coordinates for `to_frame` to subtract.
- `a_smaller_sixteen_ten_display_is_scaled_and_barred`: a 1280x800 laptop panel into a 1920x1080 canvas gives `letterbox = (96, 0, 1824, 1080)`; the panel's centre is the canvas centre `(960, 540)`, its top-left is the inner edge of the left bar `(96, 0)`, and the aspect is preserved.
- `a_point_off_the_new_display_is_clamped_to_the_canvas`: points left of, right of and above the captured display land on the canvas edge instead of outside the frame.
- `the_clamp_follows_the_canvas_origin`: on a canvas at `(-1920, -100)` the clamped edges are `(-1920, -100)` and `(-1, 979)`, not `(0, 0)` and `(1919, 1079)`.
- `a_bigger_display_scales_down_with_no_bars`: a 3840x2160 monitor into a 1920x1080 canvas halves every coordinate, so the whole of it stays visible and tracked.
