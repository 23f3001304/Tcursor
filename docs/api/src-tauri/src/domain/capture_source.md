# src-tauri/src/domain/capture_source.rs

Defines `CaptureSource`, the domain enum that identifies what screen region or target a recording session should capture. The file is pure data - no I/O, no threading, no dependencies outside this module. `Copy` semantics mean the value can be stored in settings and passed between threads without cloning.

## CaptureSource

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureSource {
    Display(u32),
    Window(u64),
    Region { x: i32, y: i32, w: u32, h: u32 },
}
```

A `Copy`-able, equality-comparable descriptor of a capture target. Three variants:

- `Display(u32)` - *Capture an entire display identified by a numeric index or platform display ID. The `u32` is the OS-assigned ID passed to the WGC monitor selector.*
- `Window(u64)` - *Capture a specific window identified by a 64-bit handle (e.g. a Windows `HWND` cast to `u64`). Enables window-scoped recording without full-screen capture.*
- `Region { x: i32, y: i32, w: u32, h: u32 }` - *Capture a sub-rectangle of the screen. `x` and `y` are signed to allow regions that start in negative coordinate space (e.g. on a left-of-primary monitor). `w` and `h` are unsigned pixel dimensions.*

### Used by

`CaptureSource` currently appears only in its own file. It is defined in the `domain` layer as the intended carrier of capture target selection once the recorder is wired to accept a configurable source rather than always capturing `Monitor::primary()`.

### Behaviors

- `region_holds_rect` - constructs `CaptureSource::Region { x: 0, y: 0, w: 800, h: 600 }` and asserts `w == 800` via pattern matching, verifying the struct variant fields round-trip correctly.
