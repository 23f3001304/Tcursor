# src-tauri/src/export/fx_state.rs

Renderer-agnostic data model and builder for per-frame click FX, spotlight, and video FX state. Provides `fx_state_at` to compute what is visually active at a given event-time, the `FxRenderer` trait shared by both GPU and CPU backends, the `select_fx` factory, and the `render` entry point the exporter calls once per output frame.

## LIFE_MS

```rust
const LIFE_MS: u32 = 600;
```

How long a single click effect stays alive, in milliseconds. Ported verbatim from the original fxdraw overlay so visual behavior is unchanged.

## FADE_MS

```rust
const FADE_MS: u32 = 250;
```

Fade-in/out duration in milliseconds for spotlight holds and video FX holds. Used by both `hold_alpha` calls in `fx_state_at`.

## FxHit

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxHit { pub x: f32, pub y: f32, pub progress: f32 }
```

One active click effect in output pixels (post-zoom). Produced by `fx_state_at` from raw mouse events and packed into the shader uniform by `build_fx_u`.

### Fields

- `x`, `y` - *center of the effect in output-frame pixel space; computed by projecting the screen-local click position through the active camera transform.*
- `progress` - *fractional age of the effect in `[0, 1]`; 0 = just clicked, 1 = fully expired. The shader scales animation timing by this value.*

### Used by

- `src-tauri/src/export/fx_uniforms.rs` - `build_fx_u` packs each `FxHit` into `FxU.hits`.
- `src-tauri/src/export/fxdraw.rs` - `CpuFx::apply` iterates hits for software rendering.
- `src-tauri/src/export/clickdraw.rs` - reads hits for software click-ring drawing.

## Spot

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub cx: f32, pub cy: f32,
    pub dim: f32, pub radius_frac: f32, pub feather_frac: f32, pub alpha: f32,
    pub mode: SpotlightMode, pub tint: [u8; 3], pub t: f32,
}
```

Active spotlight state in output pixels. Size fields are fractions of output height so the effect scales proportionally at any resolution.

### Fields

- `cx`, `cy` - *spotlight center in output pixels; equals the projected cursor position.*
- `dim` - *background dimming strength (`0..1`); 0 = no darkening, 1 = fully dark outside the lit zone.*
- `radius_frac` - *inner (fully-lit) radius as a fraction of output height.*
- `feather_frac` - *feather-band width as a fraction of output height; added to `radius_frac` to get the outer (fully-dim) radius. Clamped to minimum 0.001 during uniform packing to prevent divide-by-zero in the shader.*
- `alpha` - *overall effect strength (`0..1`); fades in on `SpotlightHoldStart` and out on `SpotlightHoldEnd` over `FADE_MS`.*
- `mode` - *spotlight visual variant; passed as a numeric id to the shader.*
- `tint` - *RGB tint applied to the lit zone in certain modes.*
- `t` - *current time in seconds; drives animated modes like `Breathing` and `Nebula`.*

### Used by

- `src-tauri/src/export/fx_uniforms.rs` - `build_fx_u` packs `Spot` fields into `FxU.b/c/d/tint`.
- `src-tauri/src/export/fxdraw.rs` - `CpuFx::apply` applies the spotlight on the software path.
- `src-tauri/src/export/fx_gpu.rs` - read indirectly via `FxState.spot` in `GpuFx::apply`.

## VideoFx

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoFx { pub mode: VideoFxMode, pub alpha: f32, pub t: f32 }
```

Active video FX triggered by a `VideoFxHoldStart` / `VideoFxHoldEnd` action pair.

### Fields

- `mode` - *which full-frame effect to apply; passed as a numeric id to the shader.*
- `alpha` - *fade alpha from the hold ramp (`0..1`).*
- `t` - *time in seconds for animated effects.*

### Used by

- `src-tauri/src/export/fx_uniforms.rs` - `build_fx_u` packs `VideoFx` into `FxU.e`.
- `src-tauri/src/export/videodraw.rs` - reads `VideoFx` fields for the CPU video FX path.

## FxState

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct FxState {
    pub style: ClickFxStyle,
    pub color: [u8; 3],
    pub intensity: f32,
    pub hits: Vec<FxHit>,
    pub spot: Option<Spot>,
    pub video: Option<VideoFx>,
}
```

Complete renderer-agnostic description of all active FX at one output frame. Built by `fx_state_at` and consumed by any `FxRenderer` implementation.

### Fields

- `style` - *click effect variant; determines the shader branch or CPU draw path.*
- `color` - *user-configured effect color as RGB bytes.*
- `intensity` - *user-configured effect strength scalar.*
- `hits` - *zero or more active click effects; always empty when `style` is `ClickFxStyle::None`.*
- `spot` - *`None` when neither the spotlight toggle nor any Spotlight effect region is active at this frame.*
- `video` - *`None` when no `VideoFxHold` pair is active at this frame.*

### Used by

- `src-tauri/src/export/fx_uniforms.rs` - `build_fx_u` converts `FxState` to the shader uniform.
- `src-tauri/src/export/fx_gpu.rs` - `GpuFx::apply` dispatches the GPU shader.
- `src-tauri/src/export/fxdraw.rs` - `CpuFx::apply` handles the software path.
- `src-tauri/src/export/videodraw.rs` - reads `FxState.video`.

## fx_state_at

```rust
pub fn fx_state_at(
    fx: &ClickFxSettings, events: &[MouseEvent], actions: &[ActionEvent],
    scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, ow: u32, oh: u32, et: u32,
) -> Option<FxState>
```

Builds the FX state at event-time `et`. Returns `None` when nothing is active so the renderer can skip the frame entirely.

### Inputs

- `fx: &ClickFxSettings` - user FX settings (spotlight toggle, dim, radius, feather, click style, color, intensity, video FX mode). *Why:* single source of all user-tunables; no hidden globals.*
- `events: &[MouseEvent]` - full mouse log. *Why:* `hits_at` scans this to find clicks within `LIFE_MS` of `et`.*
- `actions: &[ActionEvent]` - action track. *Why:* `hold_alpha` derives the video FX fade ramp from `VideoFxHoldStart`/`VideoFxHoldEnd` pairs. Spotlight is no longer hold-driven here - recorded holds are seeded as editable Spotlight regions (`edit::seed`), so the spotlight comes from the effect regions + the settings toggle.*
- `scene: &Scene` - active layout scene at `et`. *Why:* click screen coordinates must be converted to panel-local coordinates before projection into output space.*
- `cam: Camera` - camera transform at `et` (center + scale). *Why:* `project` maps panel-local coordinates to output pixels using this transform.*
- `cur: FramePoint` - cursor's pre-computed frame position. *Why:* the spotlight tracks the cursor; the exporter already computed this position so it is passed directly.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* `to_panel` needs these to normalize click coordinates into the panel rect.*
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* passed to `project` to map positions to output pixels.*
- `et: u32` - event time in ms. *Why:* all time-indexed queries (`hits_at`, `hold_alpha`) use this.*

### Returns

`Option<FxState>` - `None` when spotlight alpha is zero, no active click hits, and no video FX is active; otherwise `Some(FxState)`.

### Implementation

1. Compute `s_alpha`: if `fx.spotlight` toggle is on, start at `1.0`; take the max with `hold_alpha` from `SpotlightHoldStart`/`SpotlightHoldEnd`. *Why max:* the toggle keeps spotlight always-on at full alpha; the action-based ramp activates when the setting is off but the user pressed the hotkey.*
2. If `s_alpha > 0.0`, call `project(cur.x, cur.y, cam, ow, oh)` and build a `Spot` from `fx` settings and `et as f32 / 1000.0` for the time field.
3. If `fx.style` is not `None`, call `hits_at(events, et, LIFE_MS)` and for each hit convert from screen-local to panel-local via `to_panel`, then project to output pixels via `project`. *Why per-hit:* the camera transform differs per frame, so each hit must be projected individually.*
4. Compute video FX alpha via `hold::hold_alpha` gated on `VideoFxHoldStart`/`VideoFxHoldEnd`. Build `VideoFx` only when `va > 0.0`.
5. Return `None` when all three are empty/`None`. Otherwise return `Some(FxState)`.

### Behaviors worth knowing (unit tests)

- `nothing_active_is_none` - no events, no spotlight -> `None`.
- `spotlight_toggle_makes_a_spot_at_cursor` - spotlight enabled, no actions -> `Spot` at cursor with `alpha = 1.0` and empty `hits`.
- `a_click_makes_a_hit_in_output_space` - click at `t=0`, queried at `t=300` -> one hit with `progress > 0`.
- `style_none_suppresses_click_hits` - `ClickFxStyle::None` -> `hits` is empty even when clicks exist.
- `hold_action_ramps_spot_alpha` - `SpotlightHoldStart` at 1000ms, query at 1125ms (125ms into 250ms ramp) -> `spot.alpha ~0.5`.

## FxRenderer

```rust
pub trait FxRenderer {
    fn apply(&self, out: &mut [u8], ow: u32, oh: u32, state: &FxState);
}
```

Contract for GPU and CPU FX renderers. `out` is a composited BGRA buffer of `ow * oh * 4` bytes modified in-place. Implementations: `GpuFx` (wgpu) and `CpuFx` (software fallback).

## select_fx

```rust
pub fn select_fx(ow: u32, oh: u32) -> Box<dyn FxRenderer>
```

Factory that returns the best available renderer for output dimensions `ow x oh`. Tries the GPU path first (`gpu_available()` then `GpuFx::new`); falls back to `CpuFx` if either check fails. Called once by the exporter before the render loop.

### Inputs

- `ow: u32`, `oh: u32` - output dimensions. *Why:* both renderers need these to allocate frame-sized resources.*

### Returns

`Box<dyn FxRenderer>` - either `GpuFx` or `CpuFx`, selected at runtime.

## render

```rust
pub fn render(
    r: &dyn FxRenderer, out: &mut [u8], ow: u32, oh: u32, fx: &ClickFxSettings,
    events: &[MouseEvent], actions: &[ActionEvent], scene: &Scene, cam: Camera, cur: FramePoint,
    sw: u32, sh: u32, et: u32, keys: &HotkeySettings,
)
```

Per-frame entry point called by the exporter after compositing the base frame. Builds FX state, applies the renderer, then overlays captions.

### Inputs

- `r: &dyn FxRenderer` - the renderer selected by `select_fx`. *Why trait object:* decouples the exporter from GPU/CPU choice.*
- `out: &mut [u8]` - composited BGRA frame; modified in-place. *Why:* FX are composited on top of the already-rendered frame without a separate allocation.*
- `fx: &ClickFxSettings` - FX settings; `fx.enabled` is checked first as a fast exit. *Why early return:* when FX is fully disabled, no state is built and no renderer is invoked.*
- `events`, `actions`, `scene`, `cam`, `cur`, `sw`, `sh`, `et` - forwarded verbatim to `fx_state_at`.
- `keys: &HotkeySettings` - hotkey bindings forwarded to `caption::overlay`. *Why:* captions label hotkey actions and need the binding strings to construct the text.*

### Implementation

1. Return immediately if `!fx.enabled`.
2. Call `fx_state_at`; if `Some(state)`, call `r.apply(out, ow, oh, &state)`.
3. Call `caption::overlay(out, ow, oh, actions, keys, et, fx.captions)` unconditionally. *Why always:* captions are independent of click/spotlight FX and must appear even when FX rendering was skipped.*
