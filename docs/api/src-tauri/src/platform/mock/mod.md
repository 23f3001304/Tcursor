# src-tauri/src/platform/mock/mod.rs

`#[cfg(test)]`. A `Platform` whose four ports answer without an OS, so the recorder can be started, paused, resumed and stopped in a unit test. Before Batch D nothing about the record path could be exercised without a desktop session: `Running` held four concrete Windows types and `start_recording` called them by name.

*Why `#[cfg(test)]` and not a normal module.* Nothing in a shipping build should be able to reach a platform that cannot record. Gating it also keeps `TakeHooks::silent` and the mock structs out of the release binary without a `dead_code` allow.

*What is deliberately NOT mocked.* The filesystem. `start_take` really creates the project folder, `save_inputs` really writes `events.json` and `save_session_files` really writes `sync.json` and `project.tcursor` - the cycle test points `TakeSpec::base` at a temp directory and checks the files exist. Faking the disk too would leave the test asserting only that the mock was called.

## Calls

```rust
pub type Calls = Arc<Mutex<Vec<String>>>;
```

The shared call log every mock port appends to, in the order the recorder drives them. Shared rather than per-port because the ORDER across ports is the thing worth pinning: inputs are started before the capture on purpose (building an encoder takes time, and anything spawned after it lands late in the export), and that ordering has no other test.

## GEOMETRY

```rust
pub const GEOMETRY: CaptureGeometry;
```

The one rectangle every mock capture answer uses, 1920x1080 at the origin, so a test asserting on `ScreenInfo` has a fixed number to assert against.

## platform

```rust
pub fn platform() -> (Platform, Calls)
```

A bundle and the call log its ports write to. Returns the log alongside rather than storing it on `Platform`, because `Platform` is the real type and must not grow a test field.

## note

```rust
fn note(calls: &Calls, what: &str)
```

Appends one call name, poison-tolerant in the same `unwrap_or_else(|e| e.into_inner())` shape as every other lock site in the record path.

## MockCapture

```rust
pub struct MockCapture(Calls);
```

`CapturePort`: one display target, `GEOMETRY` for any target's bounds, and a `MockSink` from `start`. `start` logs `start:<target>`, which is what pins that `TakeSpec::target_id: None` reaches the port as `TargetId::Primary`.

## MockSink

```rust
pub struct MockSink(Calls);
```

`VideoSink`: `switch` logs the target and CALLS the first-frame hook with `GEOMETRY`, so a test of `switch_display` sees the switch record corrected the way a real capture corrects it. `stop` logs and returns two frames with timestamps, which is enough for `sync.json` to be written and for `RecordingResult::frames` to be checked.

## MockInput

```rust
pub struct MockInput(Calls);
```

`InputPort`: logs `pointer` / `hotkeys` / `cursor_shapes` and hands back streams that produce nothing.

## MockPointer

```rust
pub struct MockPointer;
```

`PointerPort`: `set_remap` is a no-op, `stop` returns no events. An empty track is a legitimate answer, not a failure - `save_inputs` writes an `events.json` with zero events, which is exactly what a take with no mouse movement produces.

## MockHotkeys

```rust
pub struct MockHotkeys;
```

`HotkeyPort`: no actions, no typing stamps.

## MockCursorShapes

```rust
pub struct MockCursorShapes;
```

`CursorShapePort`: no shape samples and an empty `CursorLayerBuilder`. Per the owner's 2026-09-15 ruling an empty bitmap layer is a degradation and never the design, so this is what a mock produces, not what a platform is allowed to.

## MockSystem

```rust
pub struct MockSystem;
```

`SystemPort`: 60 Hz, light theme, and an exclusion that always reports it stuck.

## MockAudio

```rust
pub struct MockAudio;
```

`SystemAudioPort`: `None`, the "this platform cannot capture system audio" answer. The cycle test records with system audio off, so no loopback thread is spawned at all.
