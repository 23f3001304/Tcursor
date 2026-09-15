# src-tauri/src/export/camera/jank_drive.rs

The harness that drives `CameraSim` across the synthetic scene in `jank_scene.rs` and measures what came out. Split out of that file so the scene (the fixture) and the run (the measurement) are read separately; `jank_scene.rs` includes this module and re-exports it, so the probe still calls `jscene::run`, `jscene::drive` and `jscene::band` exactly as before.

## Grid

```rust
pub enum Grid { Export, Preview }
```

The two sample grids the same camera math is driven on. Both step `t = i * 1000 / 60` in integer ms (the exporter's own timestamps land 16/17 ms apart even though frames are a uniform 16.667 ms) and hand `STEP_MS` (`1000 / 60` exactly, what `render::OUT_STEP_MS` gives the sim) to `CameraSim::step`, never `t(i) - t(i-1)`. The preview used to step a flat integer 16 ms, 4% more steps per second than the export; it now walks the export's frame index, so the two grids are one and the enum survives only to label the printouts.

## Run

```rust
pub struct Run { pub grid: Grid, pub t: Vec<u32>, pub scale: Vec<f32>, pub cx: Vec<f32>, pub cy: Vec<f32>,
    pub curx: Vec<f32>, pub cury: Vec<f32>, pub clamped: Vec<bool>, pub driver: Vec<i32> }
```

One sampled trajectory, everything the metrics need per step: the camera centre and scale, the cursor position it was fed, whether the in-frame clamp moved the centre that step, and the winning region index (`-1` for none), which mirrors the private `CameraSim::winner` so a spike can be attributed to a handoff.

## run

```rust
pub fn run(grid: Grid, cfg: &ZoomConfig, smoothness: f32) -> Run
```

Drives `Cursor::at` + `CameraSim::step` over the whole clip on `grid`, recording every sample. `clamped` is derived by comparing the centre against the half-extent at the frame edge (within 1e-3).

## micro_region

```rust
pub fn micro_region(start: u32, end: u32, follow: bool) -> ZoomRegion
```

A bare 2.2x region with the scene's own ramp lengths, for the per-hypothesis micro-probes.

## drive

```rust
pub fn drive(rs: &[ZoomRegion], cfg: &ZoomConfig, t0: u32, t1: u32, cur: impl Fn(u32) -> FramePoint)
    -> (Vec<u32>, Vec<f32>, Vec<f32>, Vec<f32>)
```

The micro-probes' `run`: a fresh sim over `[0, t1]` on the exporter's grid with a hand-written cursor function, keeping `(t, cx, cy, scale)` from `t0` on.

## vel

```rust
pub fn vel(x: &[f32]) -> Vec<f32>
```

First difference in px per frame, index-aligned with `x` (`[0]` is 0).

## band

```rust
pub fn band(ts: &[u32], v: &[f32], a: u32, b: u32) -> f32
```

Mean `|v|` over the samples whose time falls in `[a, b]` ms; 0 for an empty band.

### Used by

- `src-tauri/src/export/camera/jank_probe_tests.rs`, `jank_input_tests.rs` - the probe and its micro-probes.
- `src-tauri/src/export/camera/jank_metrics.rs` - consumes `Run`.
