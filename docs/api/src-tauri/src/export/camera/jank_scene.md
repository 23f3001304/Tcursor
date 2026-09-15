# src-tauri/src/export/camera/jank_scene.rs

The synthetic 12 s scenario behind the camera jank probe (`jank_probe_tests.rs`): a hand-made mouse track (8 ms samples with deterministic jitter, two clicks, one fast sweep, a long pause) and three zoom regions, driven straight through `Cursor` and `CameraSim` with NO renderer, so every number the probe prints is the camera math alone. Included as `mod jscene` from `camera/mod.rs` under `#[cfg(test)]`. `jank_metrics.md` turns a `Run` from here into the spike tables and the jerk summary.

## FW

`1920` - the scene's output width. `FH` is `1080`, `DUR_MS` is `12_000`.

## SWEEP

`(3000, 3400)` - the fast sweep's window in ms, where "does the camera track?" and the filter's lag are measured.

## SMOOTH_DEFAULT

`0.6` - the default `CursorSettings::smoothness`, the glide strength `Cursor` draws with.

## R1

`(2600, 5000)` - region 1's `(start_ms, end_ms)`: follow-cursor at 2.2x, 350 ms in and 450 ms out. `R2` is `(5000, 7500)`, anchored 1.8x, gapless after R1; `R3` is `(9000, 12_000)`, anchored 2.6x at a corner so the in-frame clamp bites. `ZI` (350) and `ZO` (450) are the ramp lengths, mirrored as consts so the spike classifier can name a phase boundary without re-deriving it.

## screen

```rust
pub fn screen() -> ScreenInfo
```

`FW` x `FH` at origin 0,0.

## events

```rust
pub fn events() -> Vec<MouseEvent>
```

The mouse track: the linearly interpolated waypoint path sampled every 8 ms with a +-2 px hash jitter (real `WH_MOUSE_LL` samples are never on a clean line), plus a Down/Up pair 60 ms apart at 1000 ms and 1400 ms. Sorted by time.

## regions

```rust
pub fn regions() -> Vec<ZoomRegion>
```

The three regions of the timetable above, `Easing::Smooth` both ways, layer 0.

## harness

The run harness (`Grid`, `Run`, `run`, `drive`, `micro_region`, `vel`, `band`), included from `jank_drive.rs` and re-exported here - see `docs/api/src-tauri/src/export/camera/jank_drive.md`.
