# src/editor/timeline/curveGlyphs.ts

The six named curves as small glyphs for the camera lane's transition popover (`CameraLane.tsx`, the compact buttons between two keyframes). What remained of the old `inspectors/curves.ts` once the card editor was replaced by the `EasingPicker` dropdown.

## CurveGlyph

```ts
export interface CurveGlyph { key: string; name: string; path: string }
```

`key` is the wire name the ops (`update_camera_move.easing`) and the shared `ease` (`layoutTrack.ts`, Rust `export::camera::ease`) understand; `path` depicts what the curve evaluates to.

## CURVE_GLYPHS

```ts
export const CURVE_GLYPHS: CurveGlyph[]
```

Linear, Ease in, Ease out, Ease in and out, Smooth, Spring. The spring glyph is sampled from the real oscillator (`lib/spring.ts`, at `SPRING_DEFAULT`) so it cannot lie about the curve. A new key here also needs an `ease` branch and a `valid_easing` arm.
