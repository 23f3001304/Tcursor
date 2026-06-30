# src/hud/LayoutPreview.tsx

Renders a miniature 16:9 stage preview of how the screen capture and webcam feed are composed for a given layout mode. Used inside the Preferences appearance editor to give users a live thumbnail as they adjust sizing, padding, and corner settings. This file exports one component; all geometry helpers are module-private.

## LayoutPreview

```ts
export function LayoutPreview({ mode, ma }: { mode: ModeKey; ma: ModeAppearance }): JSX.Element
```

A pure, side-effect-free component that places two absolutely-positioned divs (`.lp-screen` and `.lp-cam`) inside a `.lp-stage` container at proportional sizes and positions derived from `mode` and `ma`. Either div is omitted when the mode does not include that element (e.g., `.lp-cam` is absent in `"screen_only"` mode).

### Props

- `mode` (`ModeKey`) - selects the layout algorithm. Recognised values: `"screen"`, `"screen_only"`, `"camera"`, `"camera_only"`, `"presenter"`. *Why a string union:* matches the persisted `layout` field in `LayoutSeg` and the appearance editor's tab state, so the preview can be driven directly by those values.
- `ma` (`ModeAppearance`) - the full appearance settings object providing `pad`, `screen_size`, `screen_radius`, `cam_size`, `cam_corner`, `cam_margin_x`, `cam_margin_y`, `cam_radius`, and `cam_shape`. *Why the whole object:* the compose geometry reads many of these fields together; extracting them individually would make the prop list unwieldy.

### Behavior

The component calls the private `compose(mode, ma)` function to obtain `[screenBox | null, camBox | null]`, then converts each non-null `Box` to `React.CSSProperties` via `toStyle`, which translates 0-1 fractions to percentage strings for `left`, `top`, `width`, and `height`.

**Screen box:** border-radius is `"${ma.screen_radius * 100}%"` applied as an inline style.

**Camera box:** border-radius comes from `camBorderRadius(ma)`:
- `cam_shape === "circle"` returns `"50%"`.
- `cam_shape === "rounded"` returns `"${ma.cam_radius * 100}%"`.
- Otherwise returns `"0"` (square).

**Geometry produced by `compose` (all values are 0-1 fractions of the 16:9 stage):**

Because the stage is 16:9, a box whose width-fraction equals its height-fraction is visually 16:9. This invariant is used throughout:

- `"screen"` / `"screen_only"`: screen is centered. Width-fraction = height-fraction = `clamp((1 - 2*pad) * screen_size, 0, 1)`. In `"screen"` mode, a camera bubble is placed at the corner named by `cam_corner`, offset by `cam_margin_x` / `cam_margin_y`. `cam_size` is a height-fraction; the width-fraction is `cam_size * (9/16)` to preserve aspect ratio on the 16:9 stage.
- `"camera"` / `"camera_only"`: camera is a large centered element; height-fraction = `clamp(cam_size, 0, 1 - 2*pad)`, width-fraction = `height * (9/16)`. In `"camera"` mode a small screen inset (width-fraction 0.28, equal height-fraction) is placed bottom-left at `pad` from the edge.
- `"presenter"`: camera fills the left half and screen fills the right half, both inset by `pad`. Camera width-fraction is `camH * (9/16)`, centered horizontally in its half. Screen width-fraction equals its height-fraction (= `0.5 - pad*1.5`), vertically centered.

All computed values are clamped to [0, 1] via the module-private `clamp(v, lo, hi)` helper to prevent overflow.

### Notes

- No animations are applied; the parent `SettingsAppearance` drives re-renders by passing updated `ma` on every control change.
- The `.lp-stage` CSS class must enforce the 16:9 aspect ratio externally (via `aspect-ratio: 16/9` or padding-bottom trick); the component itself only uses percentage-based positioning.
