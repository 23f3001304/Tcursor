# src/editor/timeline/layoutTrack.ts

The preview's mirror of the export's `LayoutTrack` (`src-tauri/src/export/scene/layout.rs`): which layout is active at a given output time, cross-faded across segment boundaries, plus the shared easing evaluator every per-frame TS mirror uses. Export is the source of truth; this only drives the live canvas preview, so both must change together.

## ease

```ts
export function ease(name: string, p: number): number
```

Map progress `p` (clamped to `[0,1]`) through the curve named by an easing wire-name. **This is the only place easing is evaluated in TypeScript** - `layoutAt` below and `camMoveAt` (`stage/cameraMoves.ts`) both call it, and it is exported for exactly that reason.

It mirrors `crate::export::camera::ease`, not `export::easing::ease`: `camera::ease` is the one the export actually runs (`CameraSim`, `LayoutTrack`, `SpotlightSim`), and it is the one whose `Smooth` is smoothstep and whose `Spring` really overshoots.

- `linear` - identity.
- `spring` - ease-out-back (`k = 1.70158`): overshoots slightly past 1, then settles.
- `ease_in` / `ease_out` / `ease_in_out` - quadratic accelerate / decelerate / symmetric.
- `cubic(x1,y1,x2,y2)` - a custom bezier, delegated to `evalCubic` (`src/lib/cubicBezier.ts`), which mirrors `export::cubic::eval`.
- anything else - smoothstep (`p^2(3-2p)`), the `smooth` default. An unparseable name landing here matches `valid_easing`'s coercion on the backend, so a bad string renders as the same curve it will be stored as.

Zoom easing never reaches this function: the zoom camera curve arrives pre-baked from Rust as `camera_track`.

## layoutAt

```ts
export function layoutAt(segs: LayoutSeg[], presets: LayoutPresets | null, t: number,
  canvas: [number, number]): PreviewLayout | null
```

The active layout at output time `t`, mirroring `LayoutTrack::scene_at` exactly. Returns `null` when the presets have not loaded yet (the caller falls back to the static `PreviewLayout` from the backend).

### Rules

- A segment is active only INSIDE `[start, end)`. Outside every segment - a gap, or before the first / after the last - the base `screen` preset applies ("empty means default").
- When segments overlap, the latest-STARTING containing one wins.
- **Entry.** Within `transition_ms` of `start_ms`, the scene cross-fades from whatever was active just before that segment (`rawPresetAt(start_ms - 1)`) using the segment's own `easing`. Checked FIRST, so a segment shorter than its own two transitions still resolves deterministically - it eases in, never out.
- **Exit.** Within `transition_out_ms` of `end_ms`, it cross-fades from this segment toward whatever the track resolves AFTER it (the next segment if gapless, else the base `screen`), using `easing_out`, with the fraction reaching exactly 1 AT `end_ms` - so it lands on the successor's pose rather than jumping to it. `transition_out_ms` defaults to `0`, a hard cut, which is the historical behaviour.
- **Overlap rule.** If the successor's OWN entry blend is still running at `end_ms`, that entry WINS and the exit stands down. One blend at a time, deterministically - gapless back-to-back segments stay seamless (the successor's entry already blends FROM this segment) instead of double-blending or jumping backwards at the boundary. The exit therefore only takes visible effect falling into a gap, or into a successor that hard-cuts in.

`activeIdx` is factored out because the exit needs the same "latest-starting containing segment" lookup at `end_ms` that the main query needs at `t`.

### Interpolation

`lerpRect` mirrors the Rust `lp`: rect, radius, alpha and ring width all cross-fade continuously, but `ring_color` SNAPS to the destination's colour at the midpoint - an RGB lerp mid-transition looks muddy and the export never blends it either.

`toPreviewLayout` collapses the resolved (screen, cam) pair into the `PreviewLayout` the canvas draws, passing `canvas` through unchanged (a cross-fade never changes the backing-store size, only the panel rects inside it).

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - resolves the frame layout every composited frame.
- `src/editor/stage/CamDragHandle.tsx` - derives the PiP rect the Move-mode handle sits on, so the handle matches the drawn frame.
