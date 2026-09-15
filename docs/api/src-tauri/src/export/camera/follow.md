# src-tauri/src/export/camera/follow.rs

Where a zoom region wants the camera centred. One definition, read by **every** phase of `CameraSim::step` - the zoom-in ramp eases toward it, the hold phase damps toward it, and (since the H5 fix) so does the zoom-out ramp.

**Why one shared aim is the whole point.** The jank probe's largest measured defect (H1b, `-130` screen px/frame^2 on the 12s scene, `jank_probe_tests::input::phase::h1_ramp_in_to_hold_on_an_anchored_region`) was that the ramp-in and the hold aimed at *different things*: the ramp eased toward `r.anchor`, while the hold only ever aimed at a dead band around the LIVE camera centre. On the ramp's last step the target teleported from the anchor to the dead-band edge and the centre reversed at `k * (tx - cx)` - one frame of `-102` px after one frame of `+27`. Making both phases call the same pure function of `(region, cursor)` removes the discontinuity by construction: the aim does not depend on `self.cx`, so it cannot change when the phase does.

## REF_STEP_MS

```rust
pub(crate) const REF_STEP_MS: f32 = 1000.0 / 60.0;
```

The step length `ZoomConfig::follow_damping` was tuned at - one frame of 60fps output, 16.667ms. It is the pin that keeps the shipped feel identical: at exactly this `dt`, `damping` returns the configured value unchanged.

## damping

```rust
pub(crate) fn damping(k_ref: f32, dt_ms: f32) -> f32
```

The first-order lerp factor for a step of `dt_ms`, converted from the per-60fps-frame setting. Shared by the camera's hold/zoom-out follow AND the cursor low-pass (`Cursor::at`, `export/cursor/mod.md`) - which is the only reason the `follow` module is `pub(crate)` rather than private: one definition, so the camera and the cursor cannot drift apart on what a setting means.

**Why this exists (H4).** `follow_damping` was applied once per STEP, so its time constant depended on how often the caller stepped. The exporter steps at `1000 / out_fps` (16.667ms at 60, 33.3ms at 30) and the editor preview used to step a flat 16ms, ~4% apart, so the same setting meant a ~4% stiffer follow in the preview than in the export - the preview was quietly not previewing the export. Expressing the setting as a time constant fixes that: `1 - (1 - k)^(dt / 16.667)` is the factor whose repeated application over a given number of milliseconds is independent of how that time is divided into steps. (`camera_track` now also walks the export's own frame grid, so at 60fps the two are not merely equivalent but identical.)

### Inputs

- `k_ref: f32` - `ZoomConfig::follow_damping`, read as "fraction of the remaining error closed in one 60fps frame". Clamped to `[0, 1]`.
- `dt_ms: f32` - the **exact** frame period (`1000 / out_fps`), handed down from the frame loop through `FrameRenderer::step_camera`. Never a difference of frame timestamps: those are whole milliseconds, and differencing them is exactly the quantization this function's whole point is to avoid.

### Returns

`1 - (1 - k_ref)^(dt_ms / REF_STEP_MS)`, with the degenerate settings short-circuited: `0` never moves and `1` snaps, at any step length. A non-finite or absurd `dt_ms` falls back to / clamps toward `REF_STEP_MS` rather than producing NaN.

### Behaviors worth knowing

- `the_damping_factor_is_a_time_constant_not_a_per_step_one` (unit test): for `k` in 0.02..0.75, `damping(k, REF_STEP_MS) == k` to 1e-6 (the pin - the default feel is unchanged at 60fps), the implied time constant `-dt / ln(1 - k)` is the same to 0.1% for steps of 1..50ms, two half-frame steps compose into exactly one whole-frame step, and 0/1 stay 0/1.
- `the_export_grid_no_longer_ripples_the_damping_factor` (unit test): the end-to-end pin. In the hold phase `aim` is a fixed point, so every step multiplies the remaining error by exactly `1 - k`; that ratio is constant to 1e-6 across 20 consecutive export frames and equals the configured value. A control in the same test shows the rounded-timestamp `dt` this replaces really would have rippled (spread > 0.005). *Why it had to be fixed at the source:* the exporter's `t_ms` is `k * 1000 / 60` in INTEGER milliseconds, so DIFFERENCING it reads 16/17/17/16 even though the frames it writes are displayed a uniform 16.667ms apart - which made `k` alternate `0.0960 / 0.1020`, a ~3% per-frame ripple on tracking speed during a fast pan (sweep-only jerk rms `cx 5.63 -> 6.64`). `CameraSim::step` now takes the exact frame period from its caller instead, and the sweep is back to `5.63`.

## aim

```rust
pub(crate) fn aim(r: &ZoomRegion, cursor: FramePoint) -> (f32, f32)
```

Region `r`'s aim point for a cursor at `cursor`, in output pixels.

### Inputs

- `r: &ZoomRegion` - the region driving the camera. *Why:* `follow_cursor` picks the rule, and `anchor` is the whole answer for the anchored one.
- `cursor: FramePoint` - the current cursor in panel output pixels (`coordmap::to_panel`). *Why live, every step:* a cursor-target zoom's stored anchor is the useless screen-centre default `fromedit::anchor_for` writes.

### Returns

- **`follow_cursor` region:** the cursor itself. The region's contract is to follow the cursor, so there is no band at all - which is exactly what removes H1a, the ~290ms freeze the old camera-relative band produced when the ramp landed the centre *on* the cursor, i.e. dead in the middle of its own dead zone. The cost, measured and accepted: with the cursor parked the camera now glides the last few pixels onto it (an exponential settle with the `follow_damping` time constant) instead of stopping exactly where the ramp left it.
- **anchored region:** `r.anchor`, full stop. Until 2026-09-13 the anchor was clamped into a band of 40% of the half-viewport around the cursor ("keep the cursor in view"), so a Region zoom whose cursor wandered off slid after it - which on screen read as the zoom following the cursor, the one thing choosing Region over Cursor says it will not do (owner ruling). The frame size therefore no longer plays a part in the aim; `CameraSim::step` still clamps the *centre* into the frame afterwards, so an anchor closer to an edge than the half-viewport lands on the clamp bound instead.

### Behaviors worth knowing

- **It is its own fixed point** (`the_aim_is_its_own_fixed_point`): feeding `aim`'s own output back in as the anchor returns it unchanged. That is the property the fix rests on - a camera that the ramp parked *on* the aim has zero hold-phase error, so the first hold step moves it by nothing.
- `an_anchored_region_aims_at_its_anchor_wherever_the_cursor_is` (unit test): cursors at x 0..1919 and either frame edge on y all aim at the (1500, 540) anchor exactly.
- `a_follow_region_aims_at_the_live_cursor_and_ignores_the_anchor` (unit test).
- `an_anchored_ramp_lands_on_the_hold_pose_and_does_not_reverse` (unit test): the H1b regression gate - driven across the ramp -> hold boundary with the cursor parked 1000px away, the centre never changes sign and the worst `|dv|` stays under 6 px/frame^2 (it was ~102). The anchor sits at 674 so the whole move stays inside the in-frame clamp (at scale 2.2 a centre can go no further right than 1484); an anchor past that bound is truncated by the clamp, which is `mod phase`'s business, not this test's.
- `a_follow_ramp_keeps_tracking_across_the_hold_boundary` (unit test): the H1a gate - a cursor moving 10px/frame is still tracked at 5..11px/frame in the 150..550ms *after* the boundary (it was 0.5, a dead stop).
- `the_zoom_out_ramp_keeps_following_instead_of_freezing_the_centre` (unit test): the H5 gate - hold and zoom-out share the centre rule, so a camera mid-pan keeps better than half its speed across the ramp-out boundary (it went to a literal 0.00 before), while the scale still reaches exactly 1.0 at `end_ms`.
- Whole-scene effect (`jank_probe_tables`, export grid): jerk rms `cx 6.01 -> 3.46`, `cy 4.27 -> 1.20`; jerk max `cx 129.8 -> 31.7`, `cy 101.6 -> 12.4`. The two `-130`/`-102` spikes at t=9350 vanish from the top-10 table entirely.
