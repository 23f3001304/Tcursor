# src-tauri/src/ports/input.rs

The three input streams a take records: pointer samples, hotkey actions, and the cursor's shape plus its captured bitmaps.

*Why three ports and not one `InputPort` returning a bundle.* They have three lifetimes, three threads and three stop types, and the stop path already takes them separately. Bundling them would force a platform that can do one and not another to fail all three, which is not hypothetical: on macOS the cursor position needs no permission while clicks need Input Monitoring, and on Wayland there is no API for an unprivileged application to read the global pointer position at all - the capture stream is the only pointer source, which is why `InputPort` must be allowed to have its pointer fed by the capture.

None of the three has a `start`. All are constructed already running, so a separate `start` would only add an invalid state.

## PointerPort

```rust
pub trait PointerPort: Send
```

A running pointer tracker.

## PointerPort::set_remap

```rust
fn set_remap(&self, remap: Option<Remap>)
```

Installs (or clears) the display remap every later sample is mapped through, after a mid-take display switch. `events.json` carries one `ScreenInfo` and the export subtracts that one origin, so without the remap a click on the second monitor would be drawn on the first monitor's picture hundreds of pixels away.

`&self` and not `&mut self` because the state it writes lives behind the tracker's own lock, not in the handle, and the recorder holds the port behind a shared reference for the life of the take. `None` restores raw desktop coordinates, which is what a switch BACK to the take's own display wants: clearing the mapping rather than installing an identity one keeps the ordinary case free on the hook's hot path.

Only `PointerPort` has it. The other two streams have nothing a display switch changes.

## PointerPort::stop

```rust
fn stop(self: Box<Self>) -> Vec<MouseEvent>
```

Stops collecting and returns the take's pointer samples.

*Why no `Result`.* None of the three stops can fail today, deliberately. The Windows pointer stop is poison-tolerant on purpose: a raw unwrap there would turn "stopped collecting events" into a panicked stop command and lose the whole take's inputs, which are the cheapest thing in the take and the thing every later feature (cursor path, click effects, auto-zoom) is built from.

`self: Box<Self>` because the tracker is consumed by its stop. `Arc` could not express that without a runtime unwrap; nothing shares these objects.

## HotkeyPort

```rust
pub trait HotkeyPort: Send
```

A running hotkey tracker.

## HotkeyPort::stop

```rust
fn stop(self: Box<Self>) -> (Vec<ActionEvent>, Vec<u32>)
```

The armed chords' actions, and the typing stamps. The second vector is timestamps ONLY, never which key: that is a privacy rule the recorder is built around, not an implementation detail, and the type is what keeps it true.

## CursorShapePort

```rust
pub trait CursorShapePort: Send
```

A running cursor-shape tracker.

## CursorShapePort::stop

```rust
fn stop(self: Box<Self>) -> CursorSamples
```

The `(t_ms, CursorType)` shape log and the captured cursor layer (`events::track::cursortracker::CursorSamples`).

May legitimately come back empty. Reading another application's cursor bitmap may simply be impossible on Wayland, and `NSCursor.currentSystemCursor` reports the current process's cursor rather than the foreground app's - so the editor's System cursor style has to fall back to a bundled pack when the layer is empty, and that is an accepted product consequence rather than a failure to report.

## InputPort

```rust
pub trait InputPort: Send + Sync
```

The platform's input capture. Each method hands back a stream that is already running.

## InputPort::pointer

```rust
fn pointer(&self, move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Box<dyn PointerPort>
```

Starts pointer tracking. `move_min_interval_ms` is the move throttle the collector applies; `ledger` is the recorder's exact-span pause ledger, passed in because the tracker's own thread has no other way to reach the recorder's pause state and every sample is stamped through it.

## InputPort::hotkeys

```rust
fn hotkeys(&self, arms: Vec<Arm>, ledger: Arc<PauseTotals>) -> Box<dyn HotkeyPort>
```

Starts hotkey tracking for the chords armed from settings. Only the configured chord keys and the modifiers are inspected.

## InputPort::cursor_shapes

```rust
fn cursor_shapes(&self, ledger: Arc<PauseTotals>) -> Box<dyn CursorShapePort>
```

Starts cursor-shape tracking. Always started whatever the cursor style: the capture is cursor-free for all of them, so the real OS cursor survives only as this stream's layer, and a Hidden or Enhanced take switched to System in the editor needs it just as much as a System one does.
