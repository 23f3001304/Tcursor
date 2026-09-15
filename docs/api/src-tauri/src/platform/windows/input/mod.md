# src-tauri/src/platform/windows/input/mod.rs

MODULE OVERVIEW: `InputPort` and its three stream ports on Windows. This file is the port layer and nothing else: the four trait impls, and the re-exports that give the three handles one public spelling. The mechanism is in the four files beside it.

- `pointer.rs` - the `WH_MOUSE_LL` hook thread, the one stamp site, and the process-global `SINK`, which is PRIVATE to that file.
- `hotkeys.rs` - the `GetAsyncKeyState` poll: armed chords and privacy-safe typing stamps.
- `cursor.rs` - the 16 ms `GetCursorInfo` poll: the shape log, the captured bitmap layer, and the polled pointer position it hands to `pointer.rs`.
- `bitmap.rs` - `GetIconInfo` + `GetDIBits`, one `HCURSOR` to one RGBA sprite.

What deliberately did NOT move in here: the pause ledger arithmetic (`PauseTotals`), the shape-hold filter (`events::track::steady`), the cursor layer builder and the pixel rules (`events::track::{cursorlayer,cursorpixels}`), and the `Arm` / `Mods` / `ActionEvent` model in `actions/`. Those are what a macOS or Linux adapter reuses unchanged; only the OS calls are platform code.

Each handle is stopped BY VALUE, which is what `stop(self: Box<Self>)` is for: an `Arc` could not express that without a runtime unwrap, and nothing shares these objects.

## Win32Input

```rust
pub struct Win32Input;
```

Windows input capture: the `WH_MOUSE_LL` hook, the `GetAsyncKeyState` poll and the `GetCursorInfo` poll, each already running by the time its handle exists. A unit struct - the live state is in the three stream handles, not in the factory.

## Win32Pointer::set_remap

```rust
fn set_remap(&self, remap: Option<Remap>)
```

The `PointerPort` arm, straight through to the inherent `Win32Pointer::set_remap`, `&self` and all: the remap lives behind the hook's own lock, not in the handle.

## Win32Pointer::stop

```rust
fn stop(self: Box<Self>) -> Vec<MouseEvent>
```

Unboxes and calls the inherent stop, which posts `WM_QUIT`, joins the hook thread and takes the collector. Infallible and poison-tolerant on the other side, which is why the port has no `Result` here.

## Win32Hotkeys::stop

```rust
fn stop(self: Box<Self>) -> (Vec<ActionEvent>, Vec<u32>)
```

Unboxes and calls the inherent stop: the actions, then the typing timestamps.

## Win32CursorShapes::stop

```rust
fn stop(self: Box<Self>) -> CursorSamples
```

Unboxes and calls the inherent stop: the shape log and the layer builder, as one tuple, exactly as the stop path already takes them.

## Win32Input::pointer

```rust
fn pointer(&self, move_min_interval_ms: u32, ledger: Arc<PauseTotals>) -> Box<dyn PointerPort>
```

`Win32Pointer::start(move_min_interval_ms, ledger)`, boxed. Argument for argument what `start_recording` passes today.

## Win32Input::hotkeys

```rust
fn hotkeys(&self, arms: Vec<Arm>, ledger: Arc<PauseTotals>) -> Box<dyn HotkeyPort>
```

`Win32Hotkeys::start(arms, ledger)`, boxed. The arms are still built from settings by the caller, because which chords are armed is not a platform question.

## Win32Input::cursor_shapes

```rust
fn cursor_shapes(&self, ledger: Arc<PauseTotals>) -> Box<dyn CursorShapePort>
```

`Win32CursorShapes::start(ledger)`, boxed.
