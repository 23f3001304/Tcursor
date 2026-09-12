# src/editor/timeline/layoutTrack.ts

The preview's mirror of the export's `LayoutTrack` (`src-tauri/src/export/scene/layout.rs`): which layout is active at a given output time, cross-faded across segment boundaries, plus the shared easing evaluator every per-frame TS mirror uses. Export is the source of truth; this only drives the live canvas preview, so both must change together.

**T34 L2: zero pose math here.** A segment carrying an `Arrangement` is resolved in Rust, once, into `LayoutPresets.segs` (`src/lib/ipcPreview.ts`) - this file only picks, per segment, between that per-segment override and the segment's own preset lookup (`resolvedPanelsFor`). No pose is ever derived on the TS side; that stays the project rule (core logic once, in Rust).

## ResolvedPanels

```ts
export interface ResolvedPanels { screen: PanelRectDto; cam: PanelRectDto }
```

One resolved panel pair - either a segment's own `segs` override (a posed T34 arrangement, resolved once in Rust) or its preset's own panels. The internal currency `layoutAt` passes around instead of a full `LayoutPresetDto` (which also carries `arrangement`, unused here).

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

## fitDurations

```ts
export function fitDurations(tin: number, tout: number, span: number): [number, number]
```

TS mirror of Rust `fit_durations` (`src-tauri/src/export/camera/mod.rs`), which `LayoutTrack` applies to every segment at construction. Shrinks a segment's entry+exit proportionally so that together they fit inside its own span; a pair that already fits is returned untouched, as is `(0, 0)`.

`layoutAt` puts every segment through it (via the internal `fittedOf`) before reading either duration - the entry branch, the exit branch and the successor's "is its entry still running" test all use the fitted pair, so they cannot disagree about how long a transition is. `Math.fround` mirrors the f32 arithmetic the export truncates, so both sides land on the same millisecond.

**Why it exists.** Nothing clamped a segment's transitions against its own length, so a segment shorter than its entry never reached its own scene - while `rawSegAt` handed that unreached scene to whatever blended off it next. The frame jumped at the boundary: on a 200ms segment carrying a 350ms entry, `screen[0]` snapped `0.3409 -> 0.5000` in a single frame. An entry+exit that together outlasted the span had a second form of it, running the exit underneath the entry so the panel lurched most of the way to the successor the instant the entry expired.

## layoutAt

```ts
export function layoutAt(segs: LayoutSeg[], presets: LayoutPresets | null, t: number,
  canvas: [number, number]): PreviewLayout | null
```

The active layout at output time `t`, mirroring `LayoutTrack::scene_at` exactly. Returns `null` when the presets have not loaded yet (the caller falls back to the static `PreviewLayout` from the backend).

### Rules

- A segment is active only INSIDE `[start, end)`. Outside every segment - a gap, or before the first / after the last - the base `screen` preset applies ("empty means default").
- When segments overlap, the latest-STARTING containing one wins.
- Every "the scene at segment X" step below actually means `resolvedPanelsFor(X, presets)` (see below) - a segment's own `segs` entry when it has one, else its preset. This is true for the CURRENT segment, the entry's "from", and the exit's "to" alike, so a posed segment blends correctly on either side of a transition, not just when it's the one currently on screen.
- **Fitting.** A segment's `transition_ms`/`transition_out_ms` are first shrunk to fit its span (`fitDurations` above, mirroring what Rust's `from_segs` stores). Every rule below reads the FITTED pair, never the raw doc values.
- **Entry.** Within the fitted entry of `start_ms`, the scene cross-fades from whatever was active just before that segment (`resolvedPanelsFor(rawSegAt(start_ms - 1), presets)`) using the segment's own `easing`. Checked first; because the pair was fitted, the entry always completes inside the segment, so the segment genuinely arrives at its own scene before anything blends off it.
- **Exit.** Within the fitted exit of `end_ms`, it cross-fades from this segment toward whatever the track resolves AFTER it (the next segment if gapless, else the base `screen`), using `easing_out`, with the fraction reaching exactly 1 AT `end_ms` - so it lands on the successor's pose rather than jumping to it. `transition_out_ms` of `0` is a hard cut - what a doc saved before this field existed deserializes to, so old projects render unchanged. A segment the user creates today is seeded symmetric instead (`NEW_LAYOUT_TRANSITION_MS`, `api.md`), which is what makes a layout effect ease back OUT rather than snap.
- **Overlap rule.** If the successor's OWN entry blend is still running at `end_ms`, that entry WINS and the exit stands down. One blend at a time, deterministically - gapless back-to-back segments stay seamless (the successor's entry already blends FROM this segment) instead of double-blending or jumping backwards at the boundary. The exit therefore only takes visible effect falling into a gap, or into a successor that hard-cuts in.

`activeIdx` is factored out because the exit needs the same "latest-starting containing segment" lookup at `end_ms` that the main query needs at `t`. `rawSegAt` wraps it to return the raw `LayoutSeg` (or `null` in a gap) for `resolvedPanelsFor` to resolve.

### resolvedPanelsFor

```ts
export function resolvedPanelsFor(seg: LayoutSeg | null, presets: LayoutPresets): ResolvedPanels
```

`seg`'s resolved panels: looks up `presets.segs` by `seg.id` (a flat `.find`, since a doc's `layout` array is small) and uses that entry's `screen`/`cam` when present, else falls back to `presetOf(presets, seg.layout)`'s panels - PER PANEL, not just per segment, so a `segs` entry that only overrides one field (not produced today, but not assumed against either) still resolves something sane rather than `undefined`. `seg === null` (the gap/outside-every-segment case) always means the base `screen` preset, with no per-segment id to look up.

### Interpolation

`lerpRect` mirrors the Rust `lp`: rect, radius, alpha and ring width all cross-fade continuously, but `ring_color` SNAPS to the destination's colour at the midpoint - an RGB lerp mid-transition looks muddy and the export never blends it either.

`toPreviewLayout` collapses the resolved (screen, cam) pair into the `PreviewLayout` the canvas draws, passing `canvas` through unchanged (a cross-fade never changes the backing-store size, only the panel rects inside it).

### Used by

- `src/editor/hooks/useCompositeLoop.ts` - resolves the frame layout every composited frame.
- `src/editor/stage/CamDragHandle.tsx` - derives the PiP rect the Move-mode handle sits on, so the handle matches the drawn frame.

Both are EXPORTED as of T34 L3: stage arrange mode needs a segment's STEADY-STATE panels - what it resolves to outside any transition - to frame and to start a drag from, and re-deriving that beside `layoutAt` would be a second place for the per-segment-override-vs-preset rule to live. `useArrangeDrag` and `LayoutInspector` both call this one - as of T34 L4, so does `src/editor/timeline/layoutLane.tsx`'s `useLayoutLaneRegions`, which attaches each layout pill's resolved panels for its thumbnail (`arrThumb.ts`/`LayoutThumb.tsx`).
