# src-tauri/src/session/record/dim_guard.rs

Pure decision point for a mid-record dimension change on the GPU-native path (`gpu_frames::Cap`). The encoder is configured for one fixed `(w, h)` at recorder start (`gpu_record::GpuRecorder::start`); after a window maximize/restore/snap, or a display resolution/rotation change, or a dock/undock, WGC keeps delivering frames at the new size - it does NOT end the capture on its own (finding H1). `Cap::on_frame_arrived` needs live WGC and a real `VideoEncoder` to run at all, so this decision is split out here where it can be tested without either - the same reason `gpu_frames::record_if_encoded` exists as its own function.

## DimGuard

```rust
pub struct DimGuard {
    cfg: (u32, u32),
    fired: bool,
}
```

Latches the first mismatched frame it sees against the configured size.

- `cfg: (u32, u32)` - the encoder's configured `(width, height)`, set once at construction.
- `fired: bool` - `true` once a mismatch has been reported; gates every later call.

## DimGuard::new

```rust
pub fn new(cfg: (u32, u32)) -> Self
```

Constructs a guard over `cfg`, unfired.

## DimGuard::mismatched

```rust
pub fn mismatched(&mut self, frame: (u32, u32)) -> bool
```

True the FIRST time `frame` differs from the configured size; false on every call after that - a repeat of the same mismatch, a different mismatch, or even a frame back at the original size - since the caller has already begun ending the take.

*Why it still latches when the crate-level halt already prevents a second call in production:* `Cap::on_frame_arrived` calls `InternalCaptureControl::stop()` on a `true`, which sets the SAME halt flag `CaptureControl::stop()` (the external, user-Stop path) uses - the crate gates all future frame delivery on it, so no later frame, mismatched or not, is expected to reach the handler again. `DimGuard`'s own latch is the defensive half of that guarantee, provable without WGC or the vendored crate's internals, and is what the unit tests below pin directly.

### Implementation

1. If already fired, or `frame == cfg`, return `false`.
2. Otherwise set `fired = true` and return `true`.

### Behaviors

- `matching_dimensions_never_fire`: repeated calls with `frame == cfg` all return `false`.
- `the_first_mismatch_fires_exactly_once`: the first differently-sized frame returns `true`; a second call with that SAME new size, and a third call back at the ORIGINAL size, both return `false`.
