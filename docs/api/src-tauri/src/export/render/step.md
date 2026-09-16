# src-tauri/src/export/render/step.rs

The per-frame camera step, moved out of `render/mod.rs` (at the size cap) and given the two clocks the time remap needs (`docs/superpowers/specs/2026-09-13-time-remap-design.md`): `t_clip_abs` feeds the cursor and the raw event streams, `t_out` feeds everything the viewer sees. `walk_plan` is the one walk along a `TimeMap::frame_plan` that the exporter (`pipeline/frame_loop.rs`), the one-shot preview (`preview::walk_to`) and `camera_track` share, which is what keeps the preview's camera the export's.

## FrameRenderer::reset_camera

```rust
pub fn reset_camera(&mut self)
```

Rewinds every piece of FORWARD-ONLY per-frame state so a cached (warm) renderer can be re-used to preview an arbitrary time T by fast-forwarding `step_camera` from the start: a fresh `CameraSim` and `SpotlightSim`, `Cursor::reset()`, and `cprep.recent.clear()`.

**Why the trail history is in that list:** `CursorPrep.recent` is a 6-deep `VecDeque` that gains exactly ONE entry per `composite_at`, and `preview::render_frame` composites exactly ONE frame per call - so without clearing it, a scrub session accumulated the cursor positions of the last six SCRUB TARGETS (arbitrary, unrelated points) and `cursordraw::draw_cursor` blitted a faded sprite at each of them whenever `motion_blur > 0` (0.35 by default). Scrubbing 0:03 -> 0:40 -> 0:12 showed the real cursor plus up to five ghosts. The export is unaffected either way (it appends once per real output frame, in order) - this is a preview-only correction.

## FrameRenderer::step_camera

```rust
pub fn step_camera(&mut self, t_clip_abs: u64, t_out: u32, dt_ms: f32) -> FramePose
```

Advances the camera simulation by one frame of `dt_ms` at output time `t_out`, reading the cursor at absolute clip time `t_clip_abs`, and returns the resolved pose. Cheap: only arithmetic, no I/O. Must be called in ascending order on both clocks because `CameraSim` and the cursor index are stateful.

**Two clocks, since the time remap.** Before it, one absolute `t` produced both `ev_t = t - events_ms` and `out_t = t - video_start`, because the clip clock WAS the export's clock. Cuts and speed spans broke that identity: the viewer's clock (`t_out`, the exported file's time) now runs independently of the recording frame on screen (`t_clip_abs`). Every region list is on the output clock (`edit::remap_doc`), so the layout track, the zoom regions, the camera moves and the simulation take `t_out`; the cursor and the raw event streams take `t_clip_abs - events_ms`. `dt_ms` stays the exact output period, so the camera runs on the output clock and every per-ms filter is untouched by a speed span. `walk_plan` below is the only caller that should compute the pair.

### Inputs (what, and why it is needed)

- `t_clip_abs: u64` - capture wall-clock time of the recording frame on screen, in ms (same epoch as `RenderMeta.video_start`): `video_start + k * 1000 / fps` for recording frame `k`. Only `ev_t = t_clip_abs - events_ms` is derived from it, for the raw event streams.
- `t_out: u32` - output time in ms, `j * 1000 / fps` for output frame `j`; what every `edit.json` region is evaluated at.
- `dt_ms: f32` - the **exact** frame period, `1000 / out_fps` (`OUT_STEP_MS` = 16.666667 on every 60fps path; the exporter computes its own from the settings-resolved rate). *Why a separate argument rather than `t` minus the previous `t`:* `t` is whole milliseconds, because the frame loops build it as `k * 1000 / out_fps` in integer math. Differencing it reads 16/17/17/16 at 60fps, which is a rounding artifact and not a real timing difference - the frames are displayed a uniform 16.667ms apart. Every per-millisecond filter downstream (`follow::damping`, the handoff's velocity carry and clock, `smoothing::Damped2`) would inherit that as a ~3%/frame ripple on tracking speed during a fast pan; on the probe scene it cost `+18%` sweep-only jerk rms before the exact period was plumbed through. `t` keeps its rounded value for everything that LOOKS UP by time (region `winner`, `fit_durations`, ramp `ease` progress) - only the filters use `dt_ms`.

### Returns

`FramePose` with both clocks (`ev_t`, `out_t`), `scene` (possibly camera-shrunk), `cur` (panel-mapped cursor), and `cam` (damped camera).

### Implementation

Calls `self.cursor.at(ev_t)` for the drawn cursor position - `Cursor` now owns its event log outright (built once in `FrameRenderer::new` via `Cursor::new(events, screen, smoothness)`), so `step_camera` just delegates (passing its own `dt_ms` through for the motion lean, so it and the camera filters agree on how long a frame is) instead of replicating the index-advance + exponential low-pass inline the way an earlier version had to when `FrameRenderer` held `cur_idx`/`cur_sx`/`cur_sy`/`cur_primed` alongside a borrowed `Cursor<'a>`. Since 2026-09-14 that position is not a filter output at all but the rest/move path model's (`cursor/path.md`): `CursorSettings::smoothness_at`/`idealize_at` shape the glide between the recording's rests without moving a rest or a click, and change live via `Cursor::set_smoothness`/`set_idealize` in `reload_edit` without rebuilding the renderer. The cursor samples at `ev_t = t - events_ms` (it indexes the raw mouse log); `LayoutTrack::scene_at` and `CameraSim::step` both sample at `out_t = t - video_start`, because the layout segments and zoom regions they hold both come from `edit.json` and are therefore output-time. (`LayoutTrack::scene_at` was sampled at `ev_t` before the one-clock fix, which put every layout switch ~800 ms early in exports; the recorded-action fallback track is now shifted to output time when it is built, in `EditState::load`.) After the camera step come the CameraOnly identity override (`scene.screen.alpha < 0.5`) and then the webcam-on-zoom action (`cam_action_at` + `apply_cam_zoom_action`) when the screen panel is dominant. `cam_action_at` returns `(action, target_scale)` - the WINNING REGION'S OWN `target_scale`, not `self.cfg.target_scale` (the global default) - so `apply_cam_zoom_action`'s `zoom_progress` reaches 1.0 at the region's own peak zoom, not the global one. `self.cfg.target_scale` still feeds `self.sim.step` above (the camera framing itself, a separate concern from the webcam-shrink action). **Keyframes win while they own the frame:** the action is skipped entirely on any frame where `CameraMoveTrack::sample` returned a pose, because the override already decides the PiP there - previously the shrink ran on top of an override, silently scaling a hand-keyframed camera during zooms. Since Task 27 that is a per-frame question rather than a whole-clip one: outside the keyframes' span `sample` is `None`, so the smart shrink applies normally again. **Zoom anchors are re-anchored every frame** (`layout::anchor_frame` into `frame_regions`, from this frame's resolved scene) before `sim.step` reads them: a pinned aim lives in canvas coords, and a layout transition or display switch mid-zoom must carry it with the panel - anchoring once at the region's start left the camera zooming into where the content HAD been (2026-09-14). The legacy `camera_shrink` bool is no longer checked here; it is folded into `ZoomSettings::resolved_cam_action` (toggle off resolves to `Stay`, the identity).

**`camera_moves` override (Task 4; radius/ring fix in Task 9 Part C; span semantics in Task 27):** right after `scene` is resolved and `out_t` is computed, `step_camera` derives the LIVE layout-resolved pose from the just-resolved `scene.camera.rect` and samples `self.cam_moves` (a `CameraMoveTrack` built once from `doc.camera_moves` in `EditState::load`, refreshed by both `FrameRenderer::new` and `reload_edit`) with it:

```rust
let (ow, oh) = (self.layout.out_w as f32, self.layout.out_h as f32);
let live = Some(static_cam_pose(scene.camera.rect, ow, oh));
let cam_aspect = scene.camera.rect.w / scene.camera.rect.h.max(0.001);
if let Some(p) = self.cam_moves.sample(out_t, live) {
    scene.camera = crate::export::scene::override_camera(scene.camera, p, ow, oh, cam_aspect);
}
```

- `static_cam_pose(rect, ow, oh) -> CamPose` (`export/camera/mod.rs`) - the inverse of `rect_from_center`: converts the RESOLVED (un-overridden) camera panel's rect into a `CamPose` (center x/y + height fraction), the "what the webcam would show with zero `camera_moves`" pose. Recomputed EVERY frame from that frame's own scene, which is what makes it a *live* pose rather than a static one: if a `LayoutTrack` cross-fade is moving the panel, this moves with it.
- `CameraMoveTrack::sample(out_t, live) -> Option<CamPose>` (`export/camera/moves.rs`) - `None` for an empty track, which is the seeded-doc default, so this block never runs and the scene's camera panel is exactly whatever `overlay_for`/`resolve` produced (byte-identical to pre-Task-4 behavior). **Task 27:** `None` ALSO whenever `out_t` falls outside `[first - KF_BLEND_MS, last + KF_BLEND_MS]`, so keyframes override only their own span and layout segments own the panel everywhere else - before this, one keyframe anywhere made `sample` return `Some` for the entire clip and silently stomped every layout segment. Inside the span the track eases FROM `live` into the first keyframe over the entry window, interpolates keyframe-to-keyframe (unchanged math), and eases from the last keyframe BACK to `live` over the exit window - and because `live` is this frame's value, that exit blend tracks a layout transition that is still moving, the same principle as `CameraSim`'s driver handoff.
- `cam_aspect` - the STATIC panel's own width/height, read off `scene.camera.rect` in the same breath as `live` (i.e. before the override replaces it) and passed through `override_camera` into `rect_from_center`. *Why derived from the rect rather than re-read from `appearance.cam_aspect`:* `resolve`/`bubble_rect` already turned the setting into `width_px`/`size_px` for whichever preset (or `LayoutTrack` cross-fade between presets) is live this frame, so the rect is the resolved truth and can never disagree with the panel being overridden. Without it a single `camera_moves` keyframe squared a Wide (16:9) panel for the rest of the clip, because the pose only carries height.
- `override_camera(panel: Panel, p: CamPose, ow: f32, oh: f32, aspect: f32) -> Panel` (`export/scene/mod.rs`) - replaces `scene.camera` wholesale (not just its rect): the new rect comes from `rect_from_center` (height `h = p.size * oh`, width `w = h * aspect`), and `radius`/`ring_px` are scaled by the height ratio `new_h / old_h.max(0.001)` so a circle panel (`radius == min(w,h)/2` at its static size) stays a true circle instead of distorting toward the pre-override radius - the bug this Task 9 Part C fix corrects. `alpha`/`ring_color` are carried over unchanged.
- The override runs before the `camera_shrink` block below it, so an in-flight zoom-shrink composes on top of the overridden panel's center/radius, same as it would on top of the static one.
- Uses the same `ow`/`oh` (`self.layout.out_w`/`out_h`, `f32`) already in scope for the surrounding per-frame math - no separate output-dimension lookup.

## FrameRenderer::snap_cursor

```rust
pub fn snap_cursor(&mut self)
```

Rewind the cursor's raw index and lean (and the trail history) so the far side of the cut starts clean. `walk_plan` calls it when the plan jumps across a cut: the viewer never saw the frames in between, so a trail or a lean carried across the splice would read as the cursor sliding on its own. The camera keeps its state (a cut is a cut; the camera continuing its move across it is what every editor does). A speed-span skip is not a cut and does not snap.

## FrameRenderer::walk_plan

```rust
pub fn walk_plan(&mut self, video_start: u64, fps: u64, plan: &[u64], last_j: usize, dt_ms: f32,
                 f: impl FnMut(&mut FrameRenderer, usize, u64, &FramePose) -> bool) -> Option<FramePose>
```

Warm up over the recording frames before the first kept one (each stepped at output time 0, so the camera settles into the state output frame 0 needs on the cursor's real pre-trim path), then step every output frame `j` of `plan` through `last_j` inclusive, calling `f(renderer, j, k, pose)` after each step; `f` returning false stops the walk. Snaps the cursor on the entries of `TimeMap::plan_boundaries(fps)`, taken once before the loop and tested with `binary_search(&j)` - the question asked on the PLAN, so a cut or a non-contiguous clip join snaps on the entry that opens it and a speed-span edge does not. Asking `crosses_boundary` on output ms, as this did, snapped one entry late wherever a segment's output length was fractional; the list also never contains 0, so no `j > 0` guard is needed. Returns the last pose stepped (`None` for an empty plan). The exporter decodes and composites inside `f`; the preview passes a no-op and takes the returned pose; `camera_track` collects a sample per call.

Fingerprint: for a doc with no cuts and no speed spans the plan is `k_in ..= k_last`, `t_out == t_clip`, and the sequence of `step_camera` calls is exactly the old loop's, so an untrimmed export's frames and its `camera_track` are byte-identical to before. A TRIMMED doc differs in the warm-up only: those frames used to be stepped at their own clip time, they are stepped at output time 0 now, which is the state the first kept frame needs.

