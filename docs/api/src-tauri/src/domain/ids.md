# src-tauri/src/domain/ids.rs

Defines typed wrappers for frame indices and coordinate spaces. The file is pure data with no I/O, no threading, and no external dependencies. By making screen-space and canvas-space coordinates distinct types, the compiler rejects accidental mixing at compile time - a coord-space bug that would otherwise produce a silent wrong-value at runtime.

## FrameIndex

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameIndex(pub u64);
```

A monotonically-increasing counter identifying a frame within a recording. Wraps a `u64`; fully ordered and comparable so sequences can be sorted or range-checked. The inner value is `pub` for direct extraction.

- *Typed rather than raw `u64` so that frame counters cannot be accidentally compared to timestamps or other `u64` quantities without an explicit `.0` dereference.*

### Used by

`FrameIndex` currently appears only in its own file. It is available to any module that needs to track which frame in a sequence is being processed.

## FrameIndex::next

```rust
pub fn next(self) -> FrameIndex
```

Returns a new `FrameIndex` with value `self.0 + 1`. Does not mutate; the receiver is `Copy`-ed. No overflow guard - wraps on `u64::MAX + 1`, which is not a practical concern given recording lengths.

### Returns

`FrameIndex` with value one greater than the receiver.

### Behaviors

- `frame_index_increments` - asserts `FrameIndex(0).next() == FrameIndex(1)`.

## ScreenCoord

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenCoord { pub x: i32, pub y: i32 }
```

A pixel coordinate in screen space (signed integers, origin at the top-left of the primary monitor; negative values for positions left of or above the primary monitor).

- `x: i32` - *Horizontal screen pixel position. Signed to accommodate multi-monitor layouts where the primary monitor is not the leftmost display.*
- `y: i32` - *Vertical screen pixel position. Signed for the same reason.*

### Used by

`ScreenCoord` currently appears only in its own file. It is defined alongside `CanvasCoord` to enforce the coordinate-space distinction at the type level when the export pipeline maps screen-space click coordinates to canvas-space anchor positions.

## CanvasCoord

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasCoord { pub x: f32, pub y: f32 }
```

A coordinate in canvas space (floating-point, relative to a recorded canvas region rather than the full screen). Does not implement `Eq` because `f32` does not satisfy `Eq`.

- `x: f32` - *Horizontal canvas position in pixels or normalized units depending on the consumer.*
- `y: f32` - *Vertical canvas position.*

### Used by

`CanvasCoord` currently appears only in its own file. It is the intended target type for coordinate conversion functions (e.g. `coordmap::to_frame`) that map `ScreenCoord` click positions into frame-local positions for the autozoom anchor.

### Behaviors

- `screen_and_canvas_coords_are_distinct_types` - constructs one `ScreenCoord` and one `CanvasCoord`, asserts their respective field values; documents that the two types do not unify even though both hold x/y pairs.
