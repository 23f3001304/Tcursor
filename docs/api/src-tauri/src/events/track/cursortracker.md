# src-tauri/src/events/track/cursortracker.rs

The `GetCursorInfo` poll moved to `platform/windows/input/cursor.rs` in Batch C2. What stayed here is the shape of what one take produces, which is not a Windows fact: a macOS or Linux adapter fills the same tuple from `NSCursor.currentSystemCursor` or from the PipeWire stream's cursor metadata.

## CursorSamples

```rust
pub type CursorSamples = (Vec<(u32, CursorType)>, CursorLayerBuilder)
```

What one take's polling produced: the `(t_ms, type)` shape log, and the captured OS-cursor layer. A type alias rather than a struct because both halves are handed straight to their own persisters in `save_inputs` (`CursorTrack::save` and `CursorLayerBuilder::save`) and nothing ever holds them together.

It is the return type of `ports::input::CursorShapePort::stop`, which is why it lives outside `platform/`. Batch D left this file holding only the tuple: `Running` now holds a `Box<dyn CursorShapePort>`, so the `CursorTypeTracker` alias for the Windows adapter is gone.

See `platform/windows/input/cursor.md` for the poll loop, the classification table, the bitmap layer and the polled position it also feeds to the mouse track.
