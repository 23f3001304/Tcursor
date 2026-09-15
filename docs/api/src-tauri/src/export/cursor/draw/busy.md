# src-tauri/src/export/cursor/draw/busy.rs

Pack format v2's animated busy cursor. A v2 `pack.json` may carry `busy: { anim, fps }`; the renderer then synthesises the animation from the pack's single `busy.png`. A pack may instead ship explicit `busy_00.png .. busy_NN.png` frames, which take precedence when present. Missing `busy` is v1 behaviour: a still.

Everything here is pure math on an OUTPUT-clock timestamp, so the same instant always yields the same pose - deterministic per exported frame, and a paused preview shows exactly the frame for where the playhead sits rather than something that depends on how the user got there.

`src/editor/stage/cursor/cursorBusy.ts` is the TS mirror. Both are tested at the same five instants (0, 250, 500, 750, 1000 ms) against the same numbers; a change to one has to be made in the other in the same edit.

## BusyAnim

```rust
pub enum BusyAnim { Spin, Flip, Pulse }
```

How a pack animates its busy cursor from a single still. Serializes lowercase, which is the `pack.json` wire form.

- `Spin` - one full clockwise turn per cycle. Rings and spinners (most of the shipped packs).
- `Flip` - hold, then a 180-degree turn eased over the last 30%. Hourglasses (`classic-2001`, `pixel`).
- `Pulse` - breathe 1.0 to 1.06 and back, about the hotspot. A sleeping cat, a pocket watch (`cat`, `clay`, `glove`).

## BusySpec

```rust
pub struct BusySpec { pub anim: BusyAnim, pub fps: f32, pub frames: u32 }
```

A pack's busy animation. `anim`/`fps` come from `pack.json`; `frames` does NOT - it is filled in by `pack::count_busy_frames` scanning the folder, so a pack that ships explicit frames overrides its own declared `anim` without having to say so twice. `#[serde(default)]` on `frames` is what lets the same struct deserialize straight from `pack.json`.

*What `fps` actually does:* it sets the cycle for `Spin` (a full turn takes `24 / fps` seconds, so the shipped 24 is one turn per second) and picks frames for an explicit-frame pack. `Flip` and `Pulse` are always a 1-second cycle and ignore it.

## BusyPose

```rust
pub struct BusyPose { pub frame: u32, pub angle_deg: f32, pub scale: f32 }
```

What to draw at one instant: which frame, and the transform to apply to it about the hotspot. `frame` is always 0 for a synthesised animation; `angle_deg`/`scale` are always the identity for an explicit-frame one - the two mechanisms never combine.

## BusyPose::still

```rust
pub fn still() -> Self
```

The untransformed pose - what a v1 pack, and every non-busy cursor, always draws.

## BusyPose::is_identity

```rust
pub fn is_identity(&self) -> bool
```

Whether this pose needs the transformed blit at all. `cursordraw::draw_cursor_posed` branches on it: the identity takes the existing nearest-neighbour `blit`, bit-for-bit as before pack v2 existed (pinned by `cursordraw_tests::a_still_pose_keeps_the_nearest_neighbour_blit_and_never_interpolates`), and only a real rotation or scale reaches `cursorxform`.

## busy_pose

```rust
pub fn busy_pose(spec: &BusySpec, t_ms: u32) -> BusyPose
```

The busy pose at OUTPUT time `t_ms`.

### Implementation

1. `spec.frames > 0` - explicit frames win: `frame = floor(t_s * fps) % frames`, identity transform. `fps` is floored at 0.001 so a malformed pack cannot divide by zero.
2. `Spin` - `angle_deg = (360 * t_s * fps / 24) mod 360`, clockwise.
3. `Flip` - `flip_angle` below.
4. `Pulse` - `pulse_scale` below.

Computed in `f64` (the struct's fields stay `f32`) so the TS mirror, which has no `f32`, agrees to well under a tenth of a degree - the parity tests assert to 1e-3.

## flip_angle

```rust
fn flip_angle(t_ms: f64) -> f64
```

Still for the first 70% of the cycle (`FLIP_HOLD`), then a cosine-eased half turn over the last 30%.

*Why the completed cycles are accumulated* (`180 * n + ...`, then `rem_euclid(360)`): without it the angle would snap back to 0 at each cycle boundary, so the sprite would rock back and forth. An hourglass tips over; it does not rock. Two cycles therefore come back to 0 having gone all the way round, which `busy_tests` pins at 2000 ms.

## pulse_scale

```rust
fn pulse_scale(t_ms: f64) -> f64
```

1.0 at the cycle edges, `PULSE_PEAK` (1.06) at its middle, on `bump`'s full cosine period.

## ease_in_out

```rust
fn ease_in_out(u: f64) -> f64
```

Cosine ease 0 to 1 across `u` in 0..1 - half a cosine period, no jerk at either end. Used by `flip`.

## bump

```rust
fn bump(u: f64) -> f64
```

Cosine bump: 0 at both ends of `u` in 0..1, 1 at its middle - a full cosine period. Used by `pulse`. Kept separate from `ease_in_out` deliberately: collapsing the two (the obvious refactor) silently turns `pulse` into a ramp that ends at its peak instead of returning to rest.

### Used by

- `src-tauri/src/export/cursor/pack/cursorset.rs` - `posed`, per exported frame
- `src-tauri/src/export/cursor/pack.rs` - `busy_spec` reads it out of `pack.json`
- `src/editor/stage/cursor/cursorBusy.ts` - the TS mirror, for the canvas preview and the panel's hover grid
