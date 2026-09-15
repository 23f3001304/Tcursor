# src-tauri/src/events/collector.rs

Accumulates raw mouse events during a recording session with two deduplication filters applied before storage: time-based throttling for `Move` events, and coordinate-based deduplication that drops consecutive identical positions. Click events (`Down`, `Up`) always pass through unfiltered because they are signals of user intent and must never be silently discarded.

## EventCollector

```rust
pub struct EventCollector {
    move_min_interval_ms: u32,
    events: Vec<MouseEvent>,
    last_move_t: Option<u32>,
    last_move_xy: Option<(i32, i32)>,
}
```

Stateful buffer that applies deduplication as events arrive.

- `move_min_interval_ms` - *minimum elapsed time between consecutive accepted `Move` events; set by the caller to control disk usage vs. cursor-path fidelity.*
- `events` - *the accepted event log; grown on each `push` that passes the filters.*
- `last_move_t` - *timestamp of the last accepted `Move`; used to enforce the interval gate.*
- `last_move_xy` - *pixel coordinates of the last accepted `Move`; used to drop zero-distance moves that the OS re-emits when the cursor is stationary.*

### Used by

- `src-tauri/src/platform/windows/input/pointer.rs` - `Win32Pointer` owns a `Sink` that wraps one `EventCollector`; the hook proc calls `push` on every OS mouse event

## EventCollector::new

```rust
pub fn new(move_min_interval_ms: u32) -> Self
```

Constructs an empty collector with the given throttle interval.

### Inputs

- `move_min_interval_ms: u32` - the minimum gap between accepted `Move` events. *Why u32 ms:* matches the `MouseEvent.t` type, eliminating unit-conversion errors. Pass `0` to keep every non-duplicate move.*

### Returns

`Self` with empty `events`, `last_move_t = None`, `last_move_xy = None`.

## EventCollector::push

```rust
pub fn push(&mut self, t_ms: u32, kind: EventKind, x: i32, y: i32, button: Option<Button>)
```

Accepts or discards one incoming OS mouse event based on the configured filters, then appends it to `events` if accepted.

### Inputs

- `t_ms: u32` - recording-clock timestamp in milliseconds. *Why recording-clock not wall time:* ensures alignment with the video frame timeline, which also uses the recording start as epoch.*
- `kind: EventKind` - `Move`, `Down`, or `Up`. *Why checked first:* the two move filters apply only to `Move`; other kinds skip directly to the append.*
- `x: i32, y: i32` - screen pixel coordinates. *Why i32:* Windows `POINT` is signed; a monitor to the left of the primary has negative x.*
- `button: Option<Button>` - `Some(Left|Right)` for `Down`/`Up`; `None` for `Move`. *Why optional rather than a separate event type:* keeps `MouseEvent` to a single struct and the append path to one branch.*

### Returns

`()`. Mutation is in place; call `take` at the end of the session to retrieve the accepted events.

### Implementation

1. If `kind == Move`:
   a. If `last_move_xy == Some((x, y))`, return immediately (duplicate position). *Why drop identical coordinates:* the OS re-sends the same position on every WM_MOUSEMOVE poll tick when the cursor is idle, inflating the log without adding information.*
   b. If `last_move_t.is_some()` and `t_ms - last` < `move_min_interval_ms`, return (throttled). `saturating_sub` prevents underflow on out-of-order timestamps. *Why not also dedup on equal timestamps:* clicks sharing a timestamp with a recent move must still pass; the branch only runs for `Move`.*
   c. Update `last_move_t` and `last_move_xy`.
2. Append `MouseEvent { t: t_ms, kind, x, y, button }`.

### Behaviors

- `throttles_moves_but_keeps_clicks` - with `move_min_interval_ms=10`: a move at t=0 is kept; one at t=3 is dropped (3 < 10); one at t=12 is kept (12 >= 10); a `Down` at t=13 is kept regardless; an `Up` at t=14 is kept.
- `drops_duplicate_move_coords` - two consecutive moves to (5,5) with `interval=0` keep only the first; a later move to (6,5) is kept.

## EventCollector::take

```rust
pub fn take(self) -> Vec<MouseEvent>
```

Consumes the collector and returns the accumulated accepted events.

### Inputs

- `self` - consumed, preventing any further pushes after collection. *Why consuming rather than `&mut self`:* the session is over; consuming enforces that the collector cannot be reused accidentally.*

### Returns

`Vec<MouseEvent>` in the order they were accepted (ascending in `t`). May be empty if no events passed the filters.
