# src-tauri/src/export/preview/preview_fx.rs

The editor preview's FX overlay (spotlight + click effects) as a standalone **transparent PNG**, which the canvas loop (`useCompositeLoop`) blits over the base frame it composites in JS. The frontend has already resolved every value - the spotlight centre/radius/feather/alpha and the click hits are all in FX-canvas pixels - so this builds an `FxState` straight from the params (no events/actions/scene) and renders it with **the renderer the export would use for the same frame** (`with_fx` → `fx_state::select_fx`). Almost a pure function of its arguments, with ONE exception: the spotlight's camera-exclusion hole is gated on `PreviewSession::has_webcam` (`gate_cam_hole`), since the frontend has no reliable way to know whether `webcam.webm` exists on disk.

Splitting it out of `preview.rs`/`preview_track.rs` keeps each file focused and under the 200-line budget; its own tests live in the sibling `preview_fx_tests.rs` for the same reason.

## with_fx

```rust
pub(crate) fn with_fx<T>(ow: u32, oh: u32, f: impl FnOnce(&dyn FxRenderer) -> T) -> T
```

Runs `f` with the FX renderer for an `ow × oh` overlay, built by `fx_state::select_fx` - **the export's own selector** - and cached in a process-global `OnceLock<Mutex<...>>` keyed by that size.

*Why this exists (the parity bug):* the overlay used to be hardcoded to `CpuFx` while the export ran `select_fx`, which picks the `fx.wgsl` GPU shader on any machine with an adapter. `CpuFx` is not a pixel-mirror of that shader - Nebula and Blur spotlights are cheap approximations of it by necessity (see `spotdraw.md`), and Neon/Shockwave/Particles were simply wrong - so users saw one look while editing and a different one after exporting. Selecting the renderer the same way in both paths makes them agree by construction: GPU machines get the shader in both, adapter-less machines get `CpuFx` in both, and no third code path can drift.

*Why the renderer is MOVED out of the cache rather than borrowed under the guard:* `f` is where the render happens, and on the GPU path that is `GpuFx::apply`. A panic there while a `std::sync::Mutex` guard is held **poisons the mutex permanently** - every later `.lock()` returns `Err`, so one transient GPU fault would disable the FX overlay for the rest of the session (the export is unaffected; it never goes through here). `with_fx` therefore takes the `Box` out under a short lock, releases the guard, calls `f`, and puts the renderer back afterwards. A panic now simply drops the renderer and leaves the slot empty, so the next call rebuilds. Taking ownership also means the caller holds the renderer exclusively for the duration - which is what keeps `GpuFx`'s single reused readback buffer race-free without requiring a `Sync` bound on `FxRenderer`.

*Why the locks still use `unwrap_or_else(|e| e.into_inner())`:* defence in depth. Poisoning is unreachable through this path now, but recovering rather than propagating means a future panic elsewhere under this lock still cannot brick the overlay. Same pattern as `session/record/recorder.rs`.

*Why cached rather than per call:* `select_fx` compiles `fx.wgsl` and allocates size-dependent textures plus a readback buffer; the overlay is requested ~25×/sec (`FX_BUCKET_MS`). *Why keyed by size:* `GpuFx`'s output texture and readback buffer are allocated for exact dims, so a resized canvas must rebuild rather than reuse. *Why process-global instead of on `PreviewSession`:* it depends only on the overlay dimensions, not on which project is open, and the `Mutex` also serialises access to `GpuFx`'s single reused readback buffer. *Why the shared-device cost concern that originally justified `CpuFx` no longer applies:* `GpuFx::new` takes the process-wide `gpu::shared_device()` rather than creating its own `wgpu::Device`, so a rebuild no longer pays driver init.

## gate_cam_hole

```rust
fn gate_cam_hole(
    has_webcam: bool, cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
) -> (Option<[f32; 4]>, Option<f32>, Option<bool>)
```

Whether the spotlight's camera-exclusion hole should apply this frame. Mirrors the export's `has_hole = has_webcam && scene.camera.alpha > 0.05` gate (`fx_state.rs`'s `fx_state_at`) - the preview has no `scene.camera.alpha` to check (the frontend never resolves a `Scene`), so a requested hole (`cam_rect.is_some()`) stands in as the preview's equivalent of "a camera panel wants a hole here". Without this gate, `preview_fx_overlay` built the hole straight from the frontend-supplied `cam_rect`/`dim_camera` with no existence check at all, so a layout with an active camera panel but no recorded webcam left an un-dimmed rectangle the export never showed.

### Inputs

- `has_webcam: bool` - resolved by the caller (`preview_fx_overlay`) from `PreviewSession::has_webcam()`.
- `cam_rect`, `cam_radius`, `dim_camera` - the frontend's requested camera-exclusion params, unchanged.

### Returns

`(cam_rect, cam_radius, dim_camera)` passed through unchanged when `has_webcam` (or nothing was requested, i.e. `cam_rect` is already `None`); otherwise `(None, None, Some(true))` - the rect/radius dropped and `dim_camera` forced back to "dim everything" (belt-and-suspenders, same as `fx_state_at`'s zeroed representation), so a stray `dim_camera: false` from the frontend can never leave a hole over a webcam-less project.

## render_fx_overlay

```rust
fn render_fx_overlay(
    fx: &dyn FxRenderer, ow: u32, oh: u32, has_webcam: bool,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
) -> Result<String, String>
```

Renders the resolved FX at one preview frame and returns a `data:image/png;base64,...` overlay. Pure aside from the caller-resolved `has_webcam` bool and the injected renderer, so it's directly unit-testable without a warm `PreviewSession` or a GPU - `preview_fx_overlay` (below) is a thin `#[tauri::command]` wrapper over this.

### Inputs (what, and why it is needed)

- `fx: &dyn FxRenderer` - the renderer to draw with, supplied by the caller (`with_fx`, i.e. `select_fx`). *Why injected rather than chosen here:* it keeps this function pure enough to test with a fixed `CpuFx` on any machine, while the command still gets exactly what the export would use.
- `ow` / `oh: u32` - the FX render size in pixels (the preview canvas × `FX_SCALE`). *Why:* the overlay is produced at reduced resolution and upscaled on blit; the frontend maps hit/cursor coordinates into this same space.
- `has_webcam: bool` - see `gate_cam_hole`.
- `style: String`, `color: [u8; 3]`, `intensity: f32` - the click-FX look (lowercase style name e.g. `"ripple"`, RGB, 0..1 strength). *Why:* drives the click ripple/glow primitives, identical to `ClickFxSettings`.
- `hits: Vec<[f32; 3]>` - active click hits as `[x, y, progress]` in FX pixels. *Why:* the frontend already resolved which clicks are live and where, through the current zoom crop.
- `spot_*: Option<...>` - the resolved spotlight: centre `spot_cx/cy`, `spot_dim`, screen-scaled `spot_radius`/`spot_feather`, `spot_alpha`, lowercase `spot_mode`, `spot_tint`, and `spot_t` (seconds, for the breathing phase). *Why:* the region-override + fade resolution happens in `spotlightPreview.ts` (the mirror of `SpotlightSim`), so the backend just draws it. `None` (or `spot_alpha <= 0`) means no spotlight this frame.
- `video_*: Option<...>` - an optional full-frame video FX (mode/alpha/seconds). *Why:* symmetry with the export's `VideoFx`; unused by the current preview (always `None`).
- `cam_rect: Option<[f32; 4]>`, `cam_radius: Option<f32>`, `dim_camera: Option<bool>` - the camera panel's rect (min_x, min_y, max_x, max_y) in FX pixels, its corner radius, and the "don't dim the webcam" flag, gated through `gate_cam_hole` first. *Why:* mirrors the export's `Spot.cam_rect`/`cam_radius`/`dim_camera`; `None` (or gated off) defaults to `[0;4]`/`0.0`/`true` (dim-everything).

### Returns

`Result<String, String>` - a PNG data URL of the straight-alpha overlay, or an error string if PNG encoding fails. An empty effect yields a fully transparent PNG (harmless; the frontend skips the call when nothing is active).

### Implementation

1. `gate_cam_hole(has_webcam, cam_rect, cam_radius, dim_camera)` first.
2. Assemble an `FxState` from the params: build `Spot`/`VideoFx` only when their alpha is present and `> 0`, map the lowercase enum strings (`click_style_of`/`spot_mode_of`/`video_mode_of`), turn `hits` into `FxHit`s, and set the built `Spot`'s `cam_rect`/`cam_radius`/`dim_camera` from the (now-gated) params (defaulting as above when absent).
3. **Alpha reconstruction.** Every `FxRenderer` composites *onto an opaque frame* (multiply-dim and additive tint, in place), so there is no source alpha to read back. Render the same `FxState` twice through `fx` - once over solid **black**, once over solid **white** - then invert the over-composite per pixel: `white - black = 255·(1 - a)` per channel, so `a = 1 - (white - black)/255` (take the strongest-touched channel), and the straight colour is `black / a`. Pixels the effect never touched come out fully transparent, dim areas resolve to black-with-alpha (so the blit darkens the base), and additive click pixels resolve to bright-colour-with-alpha. *Why this still works on the GPU path:* the two renders are deterministic functions of the same uniform buffer, and the shader's only clamp is on the final colour, which is exactly the saturation the reconstruction already accounts for.
4. `png_encode` the reconstructed BGRA overlay (it preserves the alpha channel) and base64 into a data URL, reusing `preview.rs`'s helpers.

### Known preview-only gaps (not look divergences - missing inputs)

Two things the export draws never reach this command, because both are driven by the recorded **actions** stream (`ActionLog`), which the preview loop has no access to - the frontend loads clicks and cursor samples, not actions:

- **Keystroke captions** (`caption::overlay`, called by `fx_state::render` after the renderer). Beyond the missing input, captions could not simply be folded into this overlay: it is deliberately rendered at half resolution (`FX_SCALE`) and upscaled on blit, which is invisible for soft gradients but would visibly blur text.
- **Hold-triggered video FX** (`VideoFxHold` → `hold::hold_alpha`). The `video_*` parameters below exist and work, but `requestFxOverlay` has no `actions` to resolve an alpha from, so it always sends `None`.

Closing either means giving this command the warm session's actions plus an event-time clock, not adding logic to the frontend.

## preview_fx_overlay

```rust
#[tauri::command]
pub async fn preview_fx_overlay(
    ow: u32, oh: u32,
    style: String, color: [u8; 3], intensity: f32, hits: Vec<[f32; 3]>,
    spot_cx: Option<f32>, spot_cy: Option<f32>, spot_dim: Option<f32>,
    spot_radius: Option<f32>, spot_feather: Option<f32>, spot_alpha: Option<f32>,
    spot_mode: Option<String>, spot_tint: Option<[u8; 3]>, spot_t: Option<f32>,
    video_mode: Option<String>, video_alpha: Option<f32>, video_t: Option<f32>,
    cam_rect: Option<[f32; 4]>, cam_radius: Option<f32>, dim_camera: Option<bool>,
    app: tauri::AppHandle,
) -> Result<String, String>
```

Registered in `lib.rs`; the frontend calls it via `previewFxOverlay` (`src/shared/ipc.ts`) → `requestFxOverlay` (`src/editor/stage/fx/fxOverlay.ts`) - neither passes `app` explicitly, since Tauri injects `AppHandle`/`State<'_, T>` params from managed app state rather than the invoke payload, so this command's IPC signature from the frontend's side is unchanged by the `has_webcam` gate (and was unchanged again when the injected param switched from `State` to `AppHandle`). **This command being absent is exactly why the spotlight never appeared in the preview at all, historically:** the invoke rejected as an unregistered command, `fxOverlay.ts` caught the error and returned `null`, so no overlay was ever blitted.

**Off the main thread (sweep-2 Task 1) - the single biggest one.** `async fn` + `spawn_blocking`, the pattern `preview_frame` documents (`mod.md`). This is the app's hottest command: `useCompositeLoop` fires it once per `FX_BUCKET_MS` (40 ms) bucket, i.e. ~25x/sec, for the whole of playback *and* the whole of a scrub - and because `resolveSpotlight` returns `max(sim.alpha, settings_on ? 1 : 0)`, the overlay is continuously active whenever the global spotlight setting is on. Each call renders the effect twice (over black, over white - see `render_fx_overlay`), and on the GPU path each of those ends in `readback_into` -> `device.poll(Maintain::Wait)`, a full CPU-GPU fence that drains the **process-shared** wgpu device queue (`GpuFx` uses `gpu::shared_device()`, the same device the export compositor submits to). Then a 230k-pixel alpha reconstruction, a full PNG deflate and a hand-rolled base64. As a sync command every bit of that ran on the UI thread ~25 times a second - the dominant cause of the whole window feeling unresponsive during playback, and of the app getting dramatically worse the instant an export started pushing 4K work onto that shared device. `tauri::State<'_, PreviewSession>` cannot cross into `spawn_blocking`, so the injected param is now `app: tauri::AppHandle`. (Reducing the per-call cost - reusing GPU resources, a single alpha-capable pass, raw bytes instead of a data URL - is a separate change; this one only gets it off the main thread.)

*Not newly racy:* `useCompositeLoop` keeps a single-flight guard (`fxInflightRef`), so making the command async does not put two overlay renders through `with_fx` at once.

### Implementation

The whole body runs inside `tauri::async_runtime::spawn_blocking`: clamp `ow`/`oh` to at least 1 (so the `with_fx` cache key matches the size actually rendered), resolve `has_webcam` via `app.state::<PreviewSession>().has_webcam()` - now a relaxed atomic load rather than a mutex acquisition, so this 25x/sec call can never queue behind a cold renderer build (see `session.md`) - then call `render_fx_overlay` inside `with_fx` so the draw uses the export's chosen renderer. A `spawn_blocking` join failure maps to `Err(String)`, the same shape as every other failure.

### Behaviors worth knowing

- `preview_renders_with_the_exports_renderer_selection` - an overlay rendered through `with_fx` is byte-identical to the same overlay rendered with `select_fx(64, 64)` directly. The regression guard for the preview↔export FX look mismatch: it fails the moment this command goes back to a fixed renderer.
- `with_fx_rebuilds_when_the_overlay_size_changes` - two different overlay sizes in a row each render at their own size, confirming the cache is keyed rather than sticky (a stale `GpuFx` would target the wrong texture dims).
- `a_panic_inside_the_render_does_not_disable_later_overlays` - a panic raised inside `f` propagates, and the *next* `with_fx` call still renders normally. Under the old lock-across-`f` shape this test fails, and it takes the other `with_fx` tests in the same process down with it (they panic on the poisoned lock) - which is precisely the session-wide failure it guards against.
- `dim_camera_false_is_threaded_into_the_spot` - with `has_webcam: true`, the same spotlight call with `dim_camera: Some(true)` vs `Some(false)` and an identical `cam_rect` produces two different PNG data URLs, confirming the flag actually changes the rendered overlay end-to-end (IPC params -> `gate_cam_hole` -> `Spot` -> `spotdraw.rs`/`fx.wgsl`).
- `no_hole_without_webcam_even_if_dim_camera_is_false` - the same `dim_camera: false` + `cam_rect` call, but with `has_webcam: false`, renders IDENTICALLY to a plain dim with no `cam_rect` at all - the carry-forward fix this file exists to cover.
- `classic_spotlight_overlay_alpha_is_proportional_to_spot_alpha` (`preview_fx_alpha_tests.rs`, split out for the size limit) - for a Classic spotlight, the reconstructed overlay's alpha channel at `Spot.alpha = 0.5` is half its value at `1.0`, per pixel, within 8/255. This pins the exact property the editor's client-side spotlight fade rests on: because the overlay is a pure multiplicative dim of `dim * alpha * t`, the frontend can request it ONCE at `alpha = 1` and then blit it at `ctx.globalAlpha = a` every frame, producing the same pixels this command would have returned for `alpha = a` - at 60fps rather than one IPC round-trip per `FX_BUCKET_MS`. See `spotAlphaPlan` (`spotlightPreview.md`). The test runs through `with_fx`, so it pins whichever renderer `select_fx` picks on the machine, not just the CPU fallback. *It does not hold for Halo / Nebula / Blur* - their alpha response is not proportional (Halo's ring ignores alpha, the Nebula shader never reads it, Blur mixes by `t`) - which is why `ALPHA_LINEAR_SPOT_MODES` excludes exactly those three and they keep the round-trip.
