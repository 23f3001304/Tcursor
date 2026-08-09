# src/editor/inspectors/curves.ts

The shared easing-curve set for every transition-curve picker in the editor: the camera-move
keyframe inspector (`CameraMoveInspector`), the timeline segment popover (`CameraLane`), and (as
of Task 26) `ZoomInspector` and `LayoutInspector` too - both previously hand-copied their own
3-curve subset (`linear`/`smooth`/`spring`, missing `ease_in`/`ease_out`/`ease_in_out`, which the
backend has always supported for both zoom and layout easing - see `easing_from`,
`export/render/fromedit.rs`, used by both `export/scene/layout.rs` and zoom-region resolution).
Add a curve once here and it appears everywhere.

## CurveDef

```ts
export interface CurveDef { key: string; name: string; path: string; c: [number, number, number, number] }
```

One curve card: `key` is the easing wire-name, `name` the display label, `path` the SVG glyph (drawn bottom-left to top-right in a `0 -30 100 160` viewBox), `c` the cubic-bezier control points `CurveEditor` seeds its draggable handles from when you start editing that preset.

**`path` vs `c`.** `path` DEPICTS what the preset actually evaluates to; `c` is only the starting shape for a hand-edit. They agree exactly for `linear`, `ease_in`, `ease_out` and `smooth` - each of those really IS a cubic bezier, proven in `curveMath.test.ts` by evaluating `c` against `ease(key, p)` at five probe points. `ease_in_out` (piecewise quadratic) and `spring` (ease-out-back) are not single cubics, so their `c` is the nearest standard bezier; spring's overshoot handle is additionally parked at the top of the editor's visible band rather than its true `1.56`, so the handle is reachable by drag.

## CAM_CURVES

```ts
export const CAM_CURVES: CurveDef[]
```

The six selectable transition curves: `linear`, `ease_in`, `ease_out`, `ease_in_out`, `smooth`, `spring`. Each `key` MUST have a matching branch in the shared `ease` (`src/editor/timeline/layoutTrack.ts` and the Rust `export::camera::ease`) and be accepted by `valid_easing` in the Edit API - the picker only ever emits keys those already understand.

### Used by

- `src/editor/inspectors/CameraMoveInspector.tsx` - the keyframe inspector's curve cards
- `src/editor/timeline/CameraLane.tsx` - the timeline segment popover's compact curve glyphs
- `src/editor/inspectors/ZoomInspector.tsx` - the zoom block's "Transition Curve" cards (Task 26 - previously a hand-copied 3-curve array missing `ease_in`/`ease_out`/`ease_in_out`)
- `src/editor/inspectors/LayoutInspector.tsx` - the layout segment's "Transition Curve" cards (Task 26 - same fix)

All three inspectors now reach this list through `CurveEditor`, which owns the card row; `CameraLane`'s compact popover still reads `CAM_CURVES` (and `path`) directly.
