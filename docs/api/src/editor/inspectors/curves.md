# src/editor/inspectors/curves.ts

The shared easing-curve set for the camera-move transition picker, so the keyframe inspector (`CameraMoveInspector`) and the timeline segment popover (`CameraLane`) stay in lockstep - add a curve once and it appears in both.

## CurveDef

```ts
export interface CurveDef { key: string; name: string; path: string }
```

One curve card: `key` is the easing wire-name, `name` the display label, `path` the SVG glyph (drawn bottom-left to top-right in a `0 -30 100 160` viewBox).

## CAM_CURVES

```ts
export const CAM_CURVES: CurveDef[]
```

The six selectable transition curves: `linear`, `ease_in`, `ease_out`, `ease_in_out`, `smooth`, `spring`. Each `key` MUST have a matching branch in the shared `ease` (`src/editor/timeline/layoutTrack.ts` and the Rust `export::camera::ease`) and be accepted by `valid_easing` in the Edit API - the picker only ever emits keys those already understand.
