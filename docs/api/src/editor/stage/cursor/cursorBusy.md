# src/editor/stage/cursor/cursorBusy.ts

Pack format v2's animated busy cursor, mirroring Rust `src-tauri/src/export/cursor/draw/busy.rs` exactly. The export and the preview run the SAME math on the SAME output-clock timestamp, so a paused preview shows precisely the frame the export would write for that instant.

`cursorBusy.test.ts` asserts the same five instants (0, 250, 500, 750, 1000 ms) `busy_tests.rs` does, against the same numbers. A change to one side has to be made on the other in the same edit; that pair of test files is the only thing holding the two implementations together.

Three consumers: the canvas preview (`cursorPreview.ts`), the Cursor panel's pack grid (`CursorPackGrid.tsx`), and nothing else.

## BusyAnim

```ts
export type BusyAnim = "spin" | "flip" | "pulse"
```

Wire form of Rust `BusyAnim` - the lowercase names a pack's `pack.json` carries.

## BusySpec

```ts
export interface BusySpec { anim: BusyAnim; fps: number; frames: number }
```

A pack's busy animation: the declared `anim`/`fps`, plus however many explicit `busy_NN.png` frames its folder ships (`0` means `anim` synthesises the animation from the single `busy.png` instead). Arrives on `CursorPackDto.busy` for the selected pack, and on `CursorPackInfo.busy` for every pack in the grid.

## BusyPose

```ts
export interface BusyPose { frame: number; angleDeg: number; scale: number }
```

What to draw at one instant. `frame` is always 0 for a synthesised animation; `angleDeg`/`scale` are always the identity for an explicit-frame one - the two mechanisms never combine.

## STILL

```ts
export const STILL: BusyPose
```

The untransformed pose - what a v1 pack, and every non-busy cursor, always draws.

## isIdentity

```ts
export function isIdentity(p: BusyPose): boolean
```

Whether a pose needs a canvas transform at all. `drawCursorSprite` branches on it so the ordinary cursor path never pays for a `save`/`rotate`/`restore`, mirroring the Rust side's fast-path branch.

## busyPose

```ts
export function busyPose(spec: BusySpec, tMs: number): BusyPose
```

The busy pose at OUTPUT time `tMs`. The TS mirror of Rust `busy_pose`; see `busy.md` for what each animation does and why.

`tMs` may be negative (a scrub before the clip's own origin): `mod` below is a Euclidean remainder, matching Rust's `rem_euclid`, so the angle stays in 0..360 rather than going negative.

## flipAngle

```ts
function flipAngle(tMs: number): number
```

Still for the first 70% of the cycle, then a cosine-eased half turn over the last 30%, with completed cycles ACCUMULATED so it always turns the same way instead of snapping back at each boundary.

## pulseScale

```ts
function pulseScale(tMs: number): number
```

1.0 at the cycle edges, 1.06 at its middle, on `bump`'s full cosine period.

## easeInOut

```ts
function easeInOut(u: number): number
```

Cosine ease 0 to 1 across `u` in 0..1 (half a cosine period). Used by `flip`.

## bump

```ts
function bump(u: number): number
```

Cosine bump: 0 at both ends of `u` in 0..1, 1 at its middle (a full cosine period). Used by `pulse`. Kept separate from `easeInOut` deliberately - collapsing the two turns `pulse` into a ramp that ends at its peak instead of returning to rest.

## clamp01

```ts
function clamp01(u: number): number
```

Clamp to 0..1, guarding both eases against a malformed cycle fraction.

## mod

```ts
function mod(a: number, n: number): number
```

Euclidean remainder, matching Rust's `rem_euclid` for the negative times a scrub can produce. JavaScript's `%` keeps the sign of the dividend, which would put a pre-origin playhead at a negative angle.
