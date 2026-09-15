# src-tauri/src/audio/level.rs

The recorder's audio-level tap: the RMS math both capture paths use, and the lock-free slot that carries a reading out of a realtime audio callback. Feeds the HUD's wave meter through the `audio-level` event.

## AudioLevel

```rust
#[derive(Clone, Copy, serde::Serialize)]
pub struct AudioLevel { pub source: &'static str, pub rms: f32 }
```

One live reading, emitted to the frontend as `audio-level` roughly every 50ms per open source.

- `source` - `"mic"` or `"system"`, which capture produced it.
- `rms` - 0..1. The raw measurement, never a display value: the HUD does its own dB mapping (`src/shared/wave/math/level.ts`), so the two sides can be tuned independently and neither has to guess what the other assumed.

## block_rms_f32

```rust
pub fn block_rms_f32(data: &[f32]) -> f32
```

RMS of one callback's worth of f32 samples, in 0..1. Interleaved channels are treated as one block on purpose: the meter shows "how loud is this input", not a per-channel balance.

### Behaviors

- An empty block and a silent block both measure exactly `0.0`.
- A full-scale square wave measures `1.0`; a full-scale sine measures `1/sqrt(2)`.
- Out-of-range samples are clamped, not amplified - a `4.0` sample cannot report a level above full scale.

## block_rms_i16

```rust
pub fn block_rms_i16(data: &[i16]) -> f32
```

`block_rms_f32` for the i16 capture path, scaled by `i16::MAX` so both paths report the same number for the same sound - a device that hands cpal i16 and one that hands it f32 drive the meter identically.

## LevelSlot

```rust
#[derive(Default)]
pub struct LevelSlot { /* private */ }
```

A lock-free handoff from a realtime audio callback to the capture thread that owns it.

The callback may not block, allocate or emit, so it only `push`es its block's RMS here; the thread's existing 50ms poll loop `take`s the loudest block seen since the last read and emits that. **Peak-of-block-RMS** (not an average of blocks) is what a meter wants: a transient inside the window still moves the needle instead of being averaged away.

The max is a plain `fetch_max` on the f32's bit pattern, which is exact for non-negative finite floats - IEEE-754 orders them identically to their unsigned bit patterns - so there is no compare-exchange loop and no lock.

## LevelSlot::new

```rust
pub fn new() -> Self
```

An empty slot, reading silence.

## LevelSlot::push

```rust
pub fn push(&self, rms: f32)
```

Record one block's RMS. Safe to call from an audio callback: no lock, no allocation. A negative or NaN value is dropped rather than poisoning the bit-pattern comparison.

## LevelSlot::take

```rust
pub fn take(&self) -> f32
```

The loudest block since the last call, and reset to silence. Reading zero is meaningful rather than an error: the source is open and genuinely silent, which is exactly what the meter's idle state is.
