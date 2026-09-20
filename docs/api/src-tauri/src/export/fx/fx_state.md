# src-tauri/src/export/fx/fx_state.rs

Renderer-agnostic data model and builder for per-frame click FX, spotlight, and video FX state. Provides `fx_state_at` to compute what is visually active at one frame, the `FxRenderer` trait shared by both GPU and CPU backends, the `select_fx` factory, and the `render` entry point the exporter calls once per output frame. The stateful spotlight region resolver (`region_alpha`, `SpotlightSim`) is the one piece of that state which remembers the previous frame, so it lives with the rest of the spotlight in `spot/spotlight_sim.rs` (see `docs/api/src-tauri/src/export/fx/spot/spotlight_sim.md`) and is re-exported here as `fx_state::SpotlightSim`.

Everything here takes TWO times, never one: `region_t` (output clock - `EditDoc` effect regions) and `ev_t` (event clock - the raw mouse/action streams). They are named apart in every signature so a caller cannot silently pass the wrong base; on a real recording they differ by around 800 ms.

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

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` packs each `FxHit` into `FxU.hits`.
- `src-tauri/src/export/fx/fxdraw.rs` - `CpuFx::apply` iterates hits for software rendering.
- `src-tauri/src/export/fx/click/clickdraw.rs` - reads hits for software click-ring drawing.

## Spot

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub cx: f32, pub cy: f32,
    pub dim: f32, pub radius_frac: f32, pub feather_frac: f32, pub alpha: f32,
    pub mode: SpotlightMode, pub tint: [u8; 3], pub t: f32,
    pub cam_rect: [f32; 4], pub cam_radius: f32, pub dim_camera: bool,
}
```

Active spotlight state in output pixels. Size fields are fractions of output height so the effect scales proportionally at any resolution.

### Fields

- `cx`, `cy` - *spotlight center in output pixels; equals the projected cursor position.*
- `dim` - *background dimming strength (`0..1`); 0 = no darkening, 1 = fully dark outside the lit zone.*
- `radius_frac` - *inner (fully-lit) radius as a fraction of output height, ALREADY pre-scaled by the screen panel's height fraction (`fx_state_at` multiplies the setting by `scene.screen.h / oh`) so the spotlight tracks the screen the cursor is on, not the whole frame. Downstream still does `oh * radius_frac`, which now lands in screen-panel pixels.*
- `feather_frac` - *feather-band width, same screen-panel pre-scaling as `radius_frac`; added to `radius_frac` to get the outer (fully-dim) radius. Clamped to minimum 0.001 during uniform packing to prevent divide-by-zero in the shader.*
- `alpha` - *overall effect strength (`0..1`); fades in on `SpotlightHoldStart` and out on `SpotlightHoldEnd` over `FADE_MS`.*
- `mode` - *spotlight visual variant; passed as a numeric id to the shader.*
- `tint` - *RGB tint applied to the lit zone in certain modes.*
- `t` - *`region_t` in seconds; drives animated modes like `Breathing` and `Nebula`. The region clock (not the event clock) so the TS preview, which sends its output-time `now` as `spotT` to `preview_fx_overlay`, renders the same phase as the export.*
- `cam_rect` - *`[min_x, min_y, max_x, max_y]`, the active camera panel's rect in OUTPUT pixels (`scene.camera.rect` converted from x/y/w/h to a min/max box). Defines the rounded-rect region the "don't dim the webcam" exclusion applies to; meaningless when `dim_camera` is `true` (zeroed in that case - see below - rather than left stale, so a future reader that checks the rect instead of the flag still gets "no hole").*
- `cam_radius` - *the camera panel's corner radius in output pixels (`scene.camera.radius`), used as the rounding radius for the `cam_rect` exclusion. Zeroed alongside `cam_rect` when there is no hole.*
- `dim_camera` - *whether the spotlight dim also darkens the camera PiP (`ClickFxSettings::spotlight_dim_camera`). `true` = today's behavior (camera dims like everything else) AND the "no hole" representation. `false` = the shader/CPU path undoes the dim inside `cam_rect`, keeping the webcam lit while the rest of the frame still dims normally - ONLY when `fx_state_at`'s `has_webcam && scene.camera.alpha > 0.05` gate passed; otherwise `fx_state_at` forces this to `true` regardless of the user's actual setting, so a missing/invisible webcam can never leave an un-dimmed empty rectangle (in a ScreenOnly layout, after `Hide`, or in a recording with no `webcam.mp4` at all).*

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` packs `Spot` fields into `FxU.b/c/d/tint/cam`.
- `src-tauri/src/export/fx/fxdraw.rs` - `CpuFx::apply` applies the spotlight on the software path.
- `src-tauri/src/export/fx/fx_gpu.rs` - read indirectly via `FxState.spot` in `GpuFx::apply`.
- `src-tauri/src/export/fx/spot/spotdraw.rs` - `draw_spot` reads `cam_rect`/`cam_radius`/`dim_camera` to undo the dim inside the camera rect on the CPU path.
- `src-tauri/src/export/preview/preview_fx.rs` - `preview_fx_overlay` builds a `Spot` with `cam_rect`/`cam_radius`/`dim_camera` from frontend-resolved IPC params, so the editor preview matches the export exactly.

## VideoFx

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoFx { pub mode: VideoFxMode, pub alpha: f32, pub t: f32 }
```

Active video FX triggered by a `VideoFxHoldStart` / `VideoFxHoldEnd` action pair.

### Fields

- `mode` - *which full-frame effect to apply; passed as a numeric id to the shader.*
- `alpha` - *fade alpha from the hold ramp (`0..1`).*
- `t` - *`ev_t` in seconds for animated effects. Video FX is a raw hotkey hold rather than a doc region, so its phase follows the same event clock its alpha does.*

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` packs `VideoFx` into `FxU.e`.
- `src-tauri/src/export/fx/videodraw.rs` - reads `VideoFx` fields for the CPU video FX path.

## FxState

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct FxState {
    pub style: ClickFxStyle,
    pub color: [u8; 3],
    pub intensity: f32,
    pub hits: Vec<FxHit>,
    pub masks: Vec<MaskDraw>,
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
- `masks` - *the mask rectangles live at this frame, in layer order. See `FxState::masks`.*
- `spot` - *`None` when neither the spotlight toggle nor any Spotlight effect region is active at this frame.*
- `video` - *`None` when no `VideoFxHold` pair is active at this frame.*

### Used by

- `src-tauri/src/export/fx/fx_uniforms.rs` - `build_fx_u` converts `FxState` to the shader uniform.
- `src-tauri/src/export/fx/fx_gpu.rs` - `GpuFx::apply` dispatches the GPU shader.
- `src-tauri/src/export/fx/fxdraw.rs` - `CpuFx::apply` handles the software path.
- `src-tauri/src/export/fx/videodraw.rs` - reads `FxState.video`.

### FxState::masks

```rust
pub masks: Vec<crate::export::fx::fx_masks::MaskDraw>,
```

The **blur, pixelate and highlight rectangles** live at this frame, already projected to output pixels and sorted by layer (`fx_masks::masks_at`). Empty for a project with no masks, which is every recording until someone draws one.

*Why it is a `Vec` and not an `Option` like `spot`:* two spotlights at once is a lighting contradiction and `SpotlightSim` elects a winner, but two blurs over two different passwords must both be blurred. Masks compose; there is no winner to elect.

*Why it rides the FX pass.* Blur and pixelate must re-sample the composited frame, and this is the one pass that has the frame uploaded as a texture. Like `lens`, it is **not a click effect**: `fx_state_at` never sets it, `render` attaches it afterwards, and a user who turned click animations off still gets the blur over their password. The renderers see the first `MAX_MASKS` here; `render` has already painted any beyond that on the CPU.

### FxState::lens

```rust
pub lens: Option<crate::export::fx::lens::Lenses>,
```

The **glass cursor material** for this frame - the refracting lens under a `material: "glass"` pack's sprite and/or the pack-independent cursor back. `None` for a plain pack with no back, which is every recording until someone picks one. See `docs/api/src-tauri/src/export/fx/lens/mod.md`.

*Why it rides the FX pass at all.* It has to refract, which means re-sampling the composited frame - and this is the one pass that has the frame uploaded as a texture. It is not a click effect, though: `fx_state_at` never sets it (it returns `lens: None`), `render` attaches it afterwards, and a user who turned click FX off still gets their glass cursor.

*Why it is placed before the pass rather than during it.* The sprite it belongs to is blitted AFTER this pass by `cursorset::draw`, so the box is computed one step earlier by `lens::build::lenses_at` and handed in. Both sides then read the same box (`cursormorph::sprite_box`), which is what stops the glass drifting off the cursor.

### FxState::grade

```rust
pub grade: Option<crate::export::grade::GradeParams>,
```

The project's **colour grade** for this frame: the eleven parameters `export::grade::params_of` resolved from `Settings.grade`, or `None` when the document has no look. Step 4 of the composite order (spec 1.3), drawn by `grade_fx` in `fx_grade.wgsl` on the GPU path and by `grade::gradedraw::draw_grade` on the CPU path, in both cases after the masks and before the video FX.

*Why it rides the FX pass.* A grade is a remap of every output pixel, and this is the one pass that already reads and writes every output pixel with the frame bound as a texture. Adding it here costs no extra pass, no extra readback and no extra upload.

*Why it is not a click effect.* Like `lens`, `fx_state_at` never sets it; `render` attaches it afterwards from its own parameter. The grade is deliberately NOT gated on `ClickFxSettings.enabled`: turning click animations off must not remove the project's look, so `render` builds a state for a grade alone the same way it does for the lens, before the `!fx.enabled` early return.

*Why it is `Option` rather than an identity value.* `params_of` returns `None` for a document that has never picked a look, which is most of them, and that `None` is what keeps an ungraded export byte-identical to one from before the grade existed: no state is built, no shader branch is taken and `pack_grade` writes zeros.

*Why it is constant for the whole export.* A grade is a document setting and not a region, so it is resolved once in `render::render_edit::EditState::load` and carried on `FrameRenderer`, never rebuilt per frame. `reload_edit` refreshes it for free by rebuilding the whole `EditState`.

## FxState::default

```rust
impl Default for FxState {
    fn default() -> Self {
        Self { style: ClickFxStyle::None, color: [0, 0, 0], intensity: 1.0, hits: Vec::new(), spot: None, video: None, lens: None }
    }
}
```

The inert state: no click style, no hits, no spotlight, no video FX, no lens, full intensity. Every `FxState` construction site - `fx_state_at`'s own literal, `render`'s fallback arm, the preview FX overlay (`export/preview/preview_fx.rs`'s `render_fx_overlay`, which builds the state the editor's live FX preview renders), and every literal across `fxdraw.rs`'s, `fx_uniforms_tests.rs`'s, `fx_gpu_tests.rs`'s, `preview_fx_tests.rs`'s and `preview_fx_alpha_tests.rs`'s test modules - spreads `..Default::default()` and names only the fields it cares about, so a field added to the struct costs each of those sites one line instead of requiring every site to list it explicitly.

`CpuFx::apply` given the default state is a byte-identical no-op: every byte of the output buffer comes back exactly as it went in. `the_default_fx_state_is_inert_and_paints_nothing` (in `fxdraw.rs`'s test module) pins this by filling a buffer with a sentinel byte, applying the default state, and asserting every byte is still the sentinel.

## fx_state_at

```rust
pub fn fx_state_at(
    fx: &ClickFxSettings, events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion],
    scene: &Scene, cam: Camera, cur: FramePoint, screen: &ScreenInfo, has_webcam: bool,
    ow: u32, oh: u32, region_t: u32, ev_t: u32, spot_sim: &mut SpotlightSim,
) -> Option<FxState>
```

The capture dimensions are no longer parameters: a click hit is mapped with `to_panel(p, scene.src, scene.screen.rect)`, and `scene.src` already IS the sub-rect the screen panel shows - the whole canvas normally, one display switch's fitted rect after a mid-take switch. So a click recorded on the switched-to display ripples on the cropped picture instead of drifting with the black bars.

Builds the FX state for one frame. Returns `None` when nothing is active so the renderer can skip the frame entirely.

**Two clocks, deliberately separate parameters.** `region_t` is OUTPUT time (0 = first video frame) - the clock every `EditDoc` region list lives on, so it drives the `effects` regions. `ev_t` is EVENT time (relative to when the input trackers started) - the clock the raw streams are recorded on, so it drives click ripples and the hold-driven video FX. A real recording offsets the two by around 800 ms; before they were split, a single `et` meant doc-authored spotlights fired ~0.8 s early in exports while the TS preview (which is entirely output-time) showed them correctly.

### Inputs

- `fx: &ClickFxSettings` - user FX settings (spotlight toggle, dim, radius, feather, click style, color, intensity, video FX mode). *Why:* single source of all user-tunables; no hidden globals.*
- `events: &[MouseEvent]` - full mouse log. *Why:* `hits_at` scans this to find clicks within `LIFE_MS` of `ev_t`.*
- `actions: &[ActionEvent]` - action track. *Why:* `hold_alpha` derives the video FX fade ramp from `VideoFxHoldStart`/`VideoFxHoldEnd` pairs. Spotlight is no longer hold-driven here - recorded holds are seeded as editable Spotlight regions (`edit::seed`), so the spotlight comes from the effect regions + the settings toggle.*
- `effects: &[EffectRegion]` - the doc's effect regions (output time). *Why:* the spotlight is region-driven; recorded hotkey holds are seeded into this list by `edit::seed`.*
- `scene: &Scene` - active layout scene this frame. *Why:* click screen coordinates must be converted to panel-local coordinates before projection into output space.*
- `cam: Camera` - camera transform this frame (center + scale). *Why:* `project` maps panel-local coordinates to output pixels using this transform.*
- `cur: FramePoint` - cursor's pre-computed frame position. *Why:* the spotlight tracks the cursor; the exporter already computed this position so it is passed directly.*
- `screen: &ScreenInfo` - the recording's screen origin (`origin_x`/`origin_y`, virtual-desktop coordinates). *Why:* `hits_at` returns RAW `WH_MOUSE_LL` desktop coordinates - a window capture or a secondary monitor has a nonzero origin, so each hit must be converted to screen-local via `coordmap::to_frame(screen, ...)` before `to_panel`, exactly like every other raw-mouse-point consumer (`Cursor::clicks`, `Cursor::at`). Without this the FX export drew ripples in the wrong place whenever `origin_x`/`origin_y` was nonzero, while the TS preview (which always converts) was correct.*
- `has_webcam: bool` - whether `webcam.mp4` exists on disk for this recording (`FrameRenderer` computes it once via `paths.webcam().exists()` in `new`). *Why:* gates the spotlight's "keep camera lit" hole (see `Spot::dim_camera`) - without a real webcam there is nothing for the hole to protect, so drawing it would leave an un-dimmed empty rectangle.*
- `sw: u32`, `sh: u32` - source screen dimensions. *Why:* `to_panel` needs these to normalize click coordinates into the panel rect.*
- `ow: u32`, `oh: u32` - output frame dimensions. *Why:* passed to `project` to map positions to output pixels.*
- `region_t: u32` - output time in ms. *Why:* `effects` are `EditDoc` regions, and every region list in the doc is output-time. Also becomes `Spot.t` (the shader's animation phase) so the TS preview - which sends its output-time `now` as `spotT` - renders the same phase.*
- `ev_t: u32` - event time in ms. *Why:* the raw-stream queries (`hits_at` over the mouse log, `hold_alpha` over the action log, and the caption overlay in `render`) index streams recorded on the event clock.*
- `spot_sim: &mut SpotlightSim` - the stateful spotlight resolver, carried across frames. *Why:* alpha eases across a region handoff, which needs the previous frame's driver + alpha.*

### Returns

`Option<FxState>` - `None` when spotlight alpha is zero, no active click hits, and no video FX is active; otherwise `Some(FxState)`.

### Implementation

1. Compute `s_alpha` via the stateful `spot_sim: &mut SpotlightSim` (`spot_sim.resolve(effects, region_t, fx.spotlight)`): the highest-`layer` active Spotlight region wins, its alpha eases across a handoff, unioned with the flat `fx.spotlight` toggle. (Recorded hotkey holds are seeded into `effects` as regions by `edit::seed`, so regions + toggle are the only sources.)
2. If `s_alpha > 0.0`, call `project(cur.x, cur.y, cam, ow, oh)` for the center, take the winning region's style via `spot_sim.style(effects, fx)`, and build a `Spot`. **The radius/feather are pre-scaled by the screen panel's height fraction (`scene.screen.rect.h / oh`)** so the spotlight is sized to the screen the cursor is on, not the whole output frame (fixes the layout-agnostic spotlight; the preview mirrors it via `layout.screen[3]`). `has_hole = has_webcam && scene.camera.alpha > 0.05` gates the camera-exclusion hole: when `true`, `cam_rect`/`cam_radius`/`dim_camera` come from `scene.camera.rect`/`scene.camera.radius`/`fx.spotlight_dim_camera` as before; when `false`, `cam_rect`/`cam_radius` zero out and `dim_camera` is forced to `true` REGARDLESS of the user's setting - the "no hole" representation, since `dim_camera: true` makes `draw_spot`/the GPU shader skip the un-dim branch entirely.
3. If `fx.style` is not `None`, call `hits_at(events, ev_t, LIFE_MS)` and for each hit: convert the RAW desktop point to screen-local via `to_frame(screen, h.sx, h.sy)`, then screen-local to panel-local via `to_panel`, then project to output pixels via `project`. *Why per-hit:* the camera transform differs per frame, so each hit must be projected individually. *Why `to_frame` first:* `hits_at` hands back the exact `(sx, sy)` `hits_at`/`clickfx::Hit` stores, which come straight from the raw `WH_MOUSE_LL` event - virtual-desktop coordinates, not screen-local ones.*
4. Compute video FX alpha via `hold::hold_alpha(actions, ev_t, ..)` gated on `VideoFxHoldStart`/`VideoFxHoldEnd`. Build `VideoFx` only when `va > 0.0`; its `t` is `ev_t` too - unlike the spotlight it is not a doc region, so both its alpha and its phase follow the clock that drives it.
5. Return `None` when all three are empty/`None`. Otherwise return `Some(FxState)`.

### Behaviors worth knowing (unit tests)

- `nothing_active_is_none` - no events, no spotlight -> `None`.
- `spotlight_toggle_makes_a_spot_at_cursor` - spotlight enabled, no actions -> `Spot` at cursor with `alpha = 1.0` and empty `hits`; full-frame screen panel leaves `radius_frac` at its setting.
- `spotlight_radius_scales_with_screen_panel_height` - a half-height screen panel halves `radius_frac`/`feather_frac` (screen-panel-relative sizing).
- `a_click_makes_a_hit_in_output_space` - click at `t=0`, queried at `t=300` -> one hit with `progress > 0`.
- `style_none_suppresses_click_hits` - `ClickFxStyle::None` -> `hits` is empty even when clicks exist.
- `region_and_event_clocks_are_sampled_independently` - a region `[1000, 2000]` sampled at `region_t = 1500` and a click at `ev_t = 2300` (300 ms after a down at 2000) are BOTH active, and `spot.t == 1.5` - only possible when the two bases are threaded separately.
- `click_hit_origin_is_converted_before_panel_mapping` - `ScreenInfo { origin_x: 500, origin_y: 300, .. }` with a Down at the RAW desktop point `(500, 300)` (i.e. the screen's own origin) and a full-frame screen panel yields a hit at the panel's top-left corner, not at `x ~= 41.6%` across (the un-converted-origin bug).
- `spotlight_hole_disabled_without_a_real_webcam` - `has_webcam: false` (even with `camera.alpha: 1.0` and the user's `spotlight_dim_camera: false`) forces `Spot.dim_camera == true`.
- `spotlight_hole_disabled_when_the_camera_panel_is_invisible` - `has_webcam: true` but `scene.camera.alpha <= 0.05` (the default disabled panel) also forces `dim_camera == true`.
- `spotlight_hole_enabled_with_a_real_visible_webcam` - `has_webcam: true` AND `camera.alpha: 1.0` lets `dim_camera` follow the user's actual `spotlight_dim_camera` setting.
- `SpotlightSim` region resolution (`spotlight_uses_per_region_fades`, `highest_layer_region_wins_style_not_first_match`, `spotlight_handoff_eases_alpha_instead_of_jump_maxing`) now lives in `spotlight_sim.rs`'s own test module - see `docs/api/src-tauri/src/export/fx/spot/spotlight_sim.md`.
- `fx_state_tests.rs` keeps the fixtures (`scr`, `full_scene`, `fx`, `cam`, `down`) and the click tests, and declares three `#[path]` sibling modules over them, one per thing the pass carries that is not a click effect: `fx_state_mask_tests.rs` (`a_mask_survives_click_animations_being_switched_off`), `fx_state_spot_tests.rs` (the spotlight) and `fx_state_grade_tests.rs` (`a_grade_runs_with_click_animations_off_and_no_spotlight`, `no_grade_and_nothing_else_leaves_the_frame_alone`). The masks and the grade arrived from two parallel tracks and together took the single file past its 320-line budget; splitting by responsibility rather than shortening anything is the standing rule.

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
    events: &[MouseEvent], actions: &[ActionEvent], effects: &[EffectRegion], scene: &Scene, cam: Camera, cur: FramePoint, screen: &ScreenInfo, has_webcam: bool,
    region_t: u32, ev_t: u32, keys: &HotkeySettings, spot_sim: &mut SpotlightSim,
    lens: Option<Lenses>, masks: Vec<MaskDraw>, grade: Option<GradeParams>,
)
```

Per-frame entry point called by the exporter after compositing the base frame. Builds FX state, applies the renderer, then overlays captions.

**Three of the things this pass draws are not click effects**, and all three are computed by the CALLER and merged in here rather than by `fx_state_at`: the cursor `lens`, the `masks` and the `grade`. `fx_state_at` only runs when `fx.enabled`, so anything built inside it disappears when a user turns click animations off. **Turning click animations off must not remove a blur over a password**, and it must not remove the glass cursor or the project's look either. So `render` builds its state as "whatever `fx_state_at` produced, OR an otherwise-inert `FxState` if any of the three is present", then attaches all three.

### Inputs

- `r: &dyn FxRenderer` - the renderer selected by `select_fx`. *Why trait object:* decouples the exporter from GPU/CPU choice.*
- `out: &mut [u8]` - composited BGRA frame; modified in-place. *Why:* FX are composited on top of the already-rendered frame without a separate allocation.*
- `fx: &ClickFxSettings` - FX settings; `fx.enabled` is checked first as a fast exit. *Why early return:* when FX is fully disabled, no state is built and no renderer is invoked.*
- `events`, `actions`, `effects`, `scene`, `cam`, `cur`, `screen`, `has_webcam`, `sw`, `sh`, `region_t`, `ev_t`, `spot_sim` - forwarded verbatim to `fx_state_at`. `FrameRenderer::composite_at` passes `&self.cursor.screen()` (the `Cursor`'s own `ScreenInfo`, already used for its own `to_frame` conversions in `clicks`/`path::PathModel::new`) rather than storing a second copy, and `self.has_webcam` (computed once via `paths.webcam().exists()` in `FrameRenderer::new`).
- `keys: &HotkeySettings` - hotkey bindings forwarded to `caption::overlay`. *Why:* captions label hotkey actions and need the binding strings to construct the text.*
- `lens: Option<Lenses>` - the glass cursor shapes for this frame, placed by `lens::build::lenses_at` before the pass because the sprite they belong to is blitted after it. See `FxState::lens`.
- `masks: Vec<MaskDraw>` - the mask rectangles for this frame, projected by `fx_masks::masks_at`, in layer order. See `FxState::masks`.
- `grade: Option<GradeParams>` - the project's colour grade, resolved once by `EditState::load` and carried on `FrameRenderer`. Like `lens` and `masks` it is attached to the state after `fx_state_at` has run, never by it. **It is not gated on `fx.enabled`:** turning click animations off must not remove the project's look, so a `Some` here builds a state on its own even when nothing else is active, and the `!fx.enabled` early return sits AFTER the renderer is applied rather than before it. See `FxState::grade`.

### The ninth mask

The GPU uniform block holds `fx_uniforms::MAX_MASKS` (eight) masks. `render` splits the list at that point, hands the first eight to the renderer through `FxState::masks`, and paints **any beyond the eighth straight onto `out` with `maskdraw::draw_masks` before the renderer runs**. So the cap degrades instead of dropping a mask, at two costs worth knowing: a spilled mask is drawn by the CPU path even on a GPU machine, and because it lands before the pass it sits UNDER everything the pass draws - the kept masks, and the grade.

### Implementation

1. Call `fx_state_at` only when `fx.enabled`; otherwise nothing is built from the click settings.
2. Split `masks` at `MAX_MASKS` and CPU-paint the overflow onto `out`.
3. `let extra = lens.is_some() || !masks.is_empty() || grade.is_some();` - if nothing was built but there IS an extra, stand up an inert `FxState` over `Default::default()` carrying only `fx.color` and `fx.intensity`, so the renderer has something to attach them to. All three bypass `fx.enabled` for the same reason: none of them is a click animation, and a user who turned click FX off still wants their blur, their glass cursor and their look.
4. Attach `lens`, `masks` and `grade` to whichever state exists, then `r.apply(out, ow, oh, &state)`.
5. Return before the hotkey captions when `!fx.enabled` - those ARE a click-FX feature, and this is the one thing the toggle really does switch off.
6. Otherwise call `click::hotkeycap::overlay(out, ow, oh, actions, keys, ev_t, fx.captions)`. *Why `ev_t`:* captions label hotkey presses read straight from the action log, which is an event-clock stream, not a doc region.

