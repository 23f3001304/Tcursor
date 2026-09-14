# src/editor/stage/cursorTilt.ts

The live preview's mirror of Rust `export/cursor/tilt.rs` - the **motion tilt**: the very slight lean a cursor takes on when it is thrown across the screen, and the single overshoot it corrects through when it stops.

Same two filters in series (a low-pass on the cursor's own velocity, then a lightly under-damped spring chasing the angle that velocity asks for), the same constants, and the same fixed 1 ms substep grid with the leftover carried.

*Why a port rather than a value from the backend:* the camera track (`CamSample`) is sampled once per project load, while the lean is a **filter over time** that the moving preview has to advance itself, tick by tick - exactly like the motion trail. *Why it has to match Rust so precisely:* when the playhead stops, the stage draws the backend's own frame over the composite (the owner's "the export is the reference" ruling). If the live draw had been leaning differently a moment earlier, that swap would visibly twitch the cursor. `cursorTilt.test.ts` and `tilt_tests.rs` pin the two against the same five instants to 1e-3, feeding both the same `dt` **bits** (the test spells out the `f64` value Rust's `1000.0f32 / 60.0` widens to, so the comparison measures the filter and not float spelling).

## REF_W

```ts
export const REF_W = 1920;
```

The screen width every tilt speed is measured against (Rust `tilt::REF_W`), so one gesture leans the same whether the recording is 1080p or 4K. The preview works in this unit directly: `CamSample.curx` is already a fraction of the screen content, so it never needs to know the recording's pixel size - see `tiltFromCam`.

## MAX_DEG

```ts
export const MAX_DEG = 6;
```

The lean a full-speed sweep settles at when `tilt` is 1, in degrees (Rust `tilt::MAX_DEG`). The cap is on the spring's target, so the spring still swings ~16% past it on the way in.

## TiltState

```ts
export interface TiltState {
  vx: number; vy: number; angle: number; avel: number;
  px: number; py: number; acc: number; primed: boolean;
}
```

The filter's state, field for field the same as Rust's `Tilt`: smoothed velocity in reference px/ms, the spring's angle (degrees) and angular velocity (degrees/s), the previous position, the un-integrated millisecond remainder, and whether there is a previous position to difference against.

*Why a plain mutable object rather than a class or immutable state:* it is advanced once per rAF tick inside `useCompositeLoop`'s loop, which holds it in a `useRef` and must never re-render on it - the same shape as the trail array next to it.

## newTilt

```ts
export function newTilt(): TiltState
```

An upright, unprimed filter. `useCompositeLoop` creates one per stage.

## resetTilt

```ts
export function resetTilt(s: TiltState): void
```

Back to upright and unprimed, in place (Rust `Tilt::reset`). `useCompositeLoop` calls it on the same discontinuity that clears the motion trail and the spotlight sim - a seek, or a re-sync jump - so the cursor never arrives somewhere else still leaning from the gesture before it. `tiltFromCam` also calls it when the setting reaches 0 with a lean still standing.

## targetDeg

```ts
function targetDeg(vx: number, vy: number, max: number): number
```

The angle a velocity of `(vx, vy)` reference px/ms asks for, capped at `max` - the mirror of Rust `tilt::target_deg`. The sign follows the horizontal direction of travel; a purely vertical throw leans by its vertical component at half weight. The `gate` factor fades the lean in from the dead-zone edge so a cursor hovering at the threshold does not flicker between upright and leaning.

## tiltStep

```ts
export function tiltStep(s: TiltState, x: number, y: number, dtMs: number, maxDeg: number): number
```

Advance by one frame with the cursor at `(x, y)` in **reference** px, and return the new lean in degrees (clockwise-positive, the sense a canvas `rotate` turns in, so the value goes straight into the pose `drawPosed` applies).

`dtMs` non-positive - a paused dirty-redraw tick - records the position and integrates nothing, exactly as in Rust; the next real frame then differences against where the cursor actually is rather than a stale point. The accumulator is capped at 100 ms so a stalled tab or a long scrub cannot run thousands of substeps on one tick.

### Behaviors

- "matches the five pins Rust's tilt_tests.rs asserts" - the shared profile, to 1e-3.
- "leans into a fast throw and never runs away past the cap" / "comes back upright through exactly one overshoot when the cursor stops" / "stays upright for ordinary pointing, and at tilt 0" / "means the same thing at 30fps as at 60fps" / "only records the position on a tick with no elapsed time" - the same five behaviors `tilt_tests.rs` pins on the Rust side.

## maxTiltDeg

```ts
export function maxTiltDeg(tilt: number): number
```

The cap the `tilt` setting (0..1) allows, mirroring Rust `tilt::max_deg`: `MAX_DEG * clamp(tilt, 0, 1)`.

Written as a `tilt > 0` test rather than a `Math.max(0, tilt)` clamp, so a missing or `NaN` setting reads as **off**. *Why that matters only on this side:* Rust's serde default cannot hand the filter a `NaN`, while a hand-edited or truncated doc reaching the preview could - and a `NaN` cap would poison the spring's accumulated state for the rest of the session rather than just skipping one frame.

## tiltFromCam

```ts
export function tiltFromCam(s: TiltState, curx: number, cury: number, aspect: number,
  dtMs: number, tilt: number): number
```

One preview tick's lean, straight from the camera track's own cursor fractions - the only entry point `useCompositeLoop` uses.

### Inputs

- `curx`, `cury` - `CamSample`'s 0..1 position across the **screen content**, the same signal Rust derives from the smoothed cursor it feeds its own filter (`preview_track.rs` divides `pose.cur` by the screen panel rect, and `pose.cur` is `Cursor::at`'s answer mapped into that panel).
- `aspect` - that content's width over its height. *Why it is needed:* `cury` is a fraction of the panel's **height** while `curx` is a fraction of its width, so without it a vertical gesture would be measured in a different unit from a horizontal one and would lean by the wrong amount on any non-square screen.
- `dtMs` - the tick's own elapsed clip time.
- `tilt` - the setting (`CursorSettings.tilt`). `0` returns 0 and unwinds any standing lean, so dragging the slider to zero straightens the cursor instead of freezing it.

### Returns

`number` - degrees, handed to `DrawCursor.tiltDeg` and composed with the busy pose by `drawCursorSprite`.

### Behaviors

- "reads the camera track's fractions as reference pixels" - identical to calling `tiltStep` with the converted coordinates.
- "is a no-op at tilt 0 and unwinds a lean the setting just switched off".
- "leans a vertical throw at half the weight of a horizontal one".
