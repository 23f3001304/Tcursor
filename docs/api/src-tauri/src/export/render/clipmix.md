# src-tauri/src/export/render/clipmix.rs

When a clip boundary cross-dissolves, and how strongly. Spec 6.4's rule is that a clip transition must **not overlap the clock**: the outgoing clip does not keep playing under the incoming one, the way a real editor's cross-dissolve would. The output clock is the plan and the plan is one frame per output instant, so the dissolve blends the outgoing clip's LAST decoded picture - held still - into the incoming clip's own frames for `transition_in_ms`. No frame of either clip is played twice, no frame is skipped, and the exported duration is the same duration the document had with `transition_in_ms: 0`. What the viewer gets is a fade out of a freeze frame rather than two moving pictures over each other; what the document gets is a transition that costs nothing on the timeline.

This module only answers WHEN and HOW MUCH. The pixels are the display switch's own primitive (`render::screen_mix::blend_into`, `screen_mix.md`), and the held frame is `pipeline::frame_loop`'s `Pipes::clip_held`, latched at every clip join.

**The boundary is a PLAN index, not a segment's `out_start`.** On the shared clips fixture the second clip's first segment carries `out_start` 5000.0 while its first plan entry is index 49, whose output instant is 4900 ms: the two accountings drift because `out_start` accumulates fractional segment lengths while the plan counts whole frames. `FramePose.out_t` is always `j * 1000 / fps`, so a window opened at `out_start` would miss the boundary frame and the incoming clip's first frame would render undissolved. The window therefore opens at `plan_start * 1000 / fps`, which is why a `ClipMixTrack` cannot be built at load time: only a walk knows the fps, and `walk_plan` resolves it once at its top.

**A clip dissolve that coincides with a display-switch dissolve resamples through the CURRENT span's rect.** `ClipMix` carries no `prev_src`, so the blend is handed `pose.scene.src` as both source and destination. That is an identity resample in every case except the one coincidence of a clip boundary with a mid-take display switch, where the held frame's own rect would have been the right one. A stated approximation for a rare overlap, taken over carrying a second rect through the latch.

**The curve is `camera::ease`, not `easing::ease`** (ruling B4-R1). Every dissolve and move the export already renders - the display-switch span mix, the camera moves, the spotlight sim, the animated text - runs on `crate::export::camera::ease`, where `Smooth` is the smoothstep `p * p * (3 - 2p)`; `export::easing::ease`, whose `Smooth` is `1 - (1 - p)^3`, has no production caller. Taking the same function means the export's two dissolves agree with each other, and it means the preview can mirror this one exactly: `src/editor/timeline/model/layoutTrack.ts::ease` is already the smoothstep, and Task 7 pins the same numbers in TypeScript against the table below.

## ClipMix

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipMix { pub prev_clip: usize, pub prev_out_ms: u32, pub alpha: f32 }
```

A clip dissolve in flight at one output instant, carried on `FramePose.clip_mix` and `None` on every frame of every document that has no clip transition.

- `prev_clip: usize` - which clip the held picture must be from, the `Segment.clip` index the outgoing span carried. *Why an index and not just the picture:* the export's latch (`Pipes::clip_held`) stores the clip it was taken at, and comparing the two is what makes a stale latch impossible to blend.
- `prev_out_ms: u32` - the last output instant of the outgoing clip, `out_start_ms - 1`. *Why it is here at all:* the streaming export has the picture already and never reads this, but the one-shot preview has no latch and seek-decodes instead - the same division of labour `SpanMix.hold_ms` makes for the display switch. It does NOT decode at `map.clip_of(prev_out_ms)`: the continuous inverse can answer the source frame AFTER the one the walk was standing on. Since Task 5 the preview runs this instant through `preview::compose::latched_ms`, `plan[prev_out_ms * fps / 1000] * 1000 / fps`, which is the frame the walk actually latched (ruling B4-R16), and the live canvas mirrors that same lookup in `preseekAt` (`src/editor/stage/clips/clipDissolve.md`).
- `alpha: f32` - the INCOMING clip's weight: 0 on the boundary frame, where the viewer still sees the outgoing clip whole, rising on the document's motion easing to 1 at the end of the window. That is exactly the sense `screen_mix::blend_into` takes (`alpha` weights `cur`), so the value feeds it with no conversion.

## ClipDissolve

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipDissolve { pub out_start_ms: u32, pub prev_clip: usize, pub dur_ms: u32 }
```

One resolved window: from `out_start_ms` (the output instant of the incoming clip's first PLAN entry) for `dur_ms` (the incoming clip's `transition_in_ms`), dissolving from clip `prev_clip`. Integer milliseconds throughout, and `Eq`, because these are the plan's own instants rather than derived floats and a test can compare them exactly.

## ClipMixTrack

```rust
pub struct ClipMixTrack { clips: Vec<Clip>, list: Vec<ClipDissolve>, easing: Easing }
```

The document's clip list, the windows resolved out of it, and the curve they run on. Lives on `EditState` (built by `EditState::load`), is moved onto `FrameRenderer` and refreshed by `reload_edit`, and is read once per frame by `step_camera`.

`new(easing)` takes the curve and nothing else: the document's `settings.motion.easing`, parsed by `fromedit::easing_from` with `Easing::Smooth` as the fallback, which is what the seeded default resolves to anyway.

`set_clips(clips)` takes the RAW document's `clips`, before `edit::remap_doc` runs. *Why raw:* `remap_doc` clears the list once the map has consumed it, so the output document has no clips to read a `transition_in_ms` off. It clears any resolved windows, because a new clip list invalidates them.

`resolve(map, fps)` is the one place the fps is known. It walks `TimeMap::clip_spans(fps)` in adjacent pairs and emits a window for each pair whose INCOMING clip carries a non-zero `transition_in_ms`, opening at that span's `plan_start * 1000 / fps`. Idempotent: it clears before it fills, so calling it twice leaves one window and calling it again at a different fps replaces the window with the one that rate implies. Called from `walk_plan`, once, before the walk. A clip that contributed no frames has no span, so it can neither open a window nor be dissolved from.

The stored length is not taken as written: the window is `transition_in_ms` CLAMPED to the incoming span's own output length, `plan_len * 1000 / fps`, so a dissolve can never outlive the clip it runs into (ruling B4-R20). On the clips fixture at 10 fps the second clip's span is 21 plan entries, 2100 ms, so a stored 5000 ms resolves to `[4900, 7000)` rather than to a window running 2900 ms past the end of the take. A window whose clamp lands on zero is not pushed at all, exactly as a stored zero is not. The two preview mirrors clamp with the same expression (`clipDissolve.md`), so all three renderers open and close the window on the same two instants.

`dissolves()` is the resolved list, in output order, for tests and for anything that wants the table rather than a lookup.

`is_empty()` is true when nothing dissolves anywhere - no clips, or clips with no transitions - which is the overwhelmingly common document and the state in which every `at` below returns `None`.

`at(out_t)` finds the window containing `out_t` and returns the `ClipMix` for it. The window is HALF OPEN, `[out_start_ms, out_start_ms + dur_ms)`: the boundary frame is inside it with `alpha` 0 and the frame at the far end is outside it, so the incoming clip is shown undiluted from there on rather than at some rounded-up 0.998. The first clip's own `transition_in_ms` is stored and ignored: there is nothing before it to dissolve from.

**Two windows cannot overlap, and the clamp in `resolve` is what makes that true.** `at` is a plain `Iterator::find` over `list`, filled in the plan order `clip_spans` returns, so it answers with whichever window it holds FIRST: an over-long window MASKED the next join's own dissolve, returning the wrong `prev_clip` for the whole of the next transition. `clipops::clamp_transition` is not the guard it looks like. It caps a `transition_in_ms` WRITE at 2 s and at half the shorter of the clip's two neighbouring output lengths, but it only runs when `update_clip` actually carries a `transition_in_ms`, so `ClipLane`'s edge drag (which sends the two source bounds and nothing else) leaves an old dissolve standing over a clip that was just dragged short; and it exempts index 0 outright, so a 2 s dissolve parked on the first clip becomes an over-long window the moment that clip is moved later. Both are ordinary editing, not a hand-edited `edit.json`. The export survived the overrun by accident - `frame_loop`'s `*c == m.prev_clip` check fails once the next join has re-latched, so the blend was silently DROPPED there - while both previews, which have no latch to compare against, kept painting the earlier clip's tail over the clip after next. Clamping the window where it is built fixes the masking and the ghost in one expression and keeps the three renderers identical.

### Behaviors

- `a_document_with_no_transition_has_no_dissolve_at_all` - the clips fixture split into two clips with `transition_in_ms: 0` resolves to an empty list, and `at` is `None` on both sides of the boundary and everywhere else.
- `the_window_opens_on_the_plan_boundary_not_on_the_segments_out_start` - a 500 ms transition on the second clip gives exactly one `ClipDissolve { out_start_ms: 4900, prev_clip: 0, dur_ms: 500 }`, at plan index 49's instant and not at the segment's 5000 ms `out_start`; 4899 is outside, 5400 is outside (half open), and 4900 is inside with `prev_out_ms` 4899 and `alpha` 0.
- `the_alpha_is_the_incoming_clips_weight_on_the_documents_curve` - on `Linear` the mid point is 0.5 and the last frame of the window is 0.998; on `Smooth` a quarter of the way in is 0.15625 (`0.25 * 0.25 * (3 - 0.5)`, where linear would read 0.25) and the mid point is still 0.5. The pinned contract Task 7 mirrors in TypeScript.
- `the_first_clips_transition_is_stored_and_ignored` - a transition on clip 0 leaves the list empty.
- `a_transition_longer_than_its_own_clip_is_clamped_to_the_incoming_span` - 5000 ms stored on the fixture's second clip resolves to `dur_ms` 2100, the span's own 21 plan entries at 10 fps; the mid point 5950 reads 0.5 and 7000, the end of the take, is outside the window. 500 ms, which fits, is stored as it is.
- `an_over_long_window_no_longer_masks_the_next_joins_own_dissolve` - three clips of 3000 ms each with 5000 ms on clip 1 and 400 ms on clip 2 resolve to `[3000, 6000)` from clip 0 and `[6000, 6400)` from clip 1; `at(6100)` answers the SECOND join (`prev_clip` 1, `prev_out_ms` 5999, alpha 0.25), where the unclamped list answered clip 0.
- `resolving_again_replaces_the_window_rather_than_appending` - two `resolve` calls at 10 fps leave one window; a third at 60 fps still leaves one, and its `out_start_ms` is the 60 fps plan's own boundary instant.
- `a_document_with_no_clips_never_dissolves` - the plain fixture has one span, no pair, and therefore no window at all, which is the guard the whole batch rests on.

### Used by

- `src-tauri/src/export/render/render_edit.rs` - `EditState.clip_mix`, built from the raw doc.
- `src-tauri/src/export/render/step.rs` - `walk_plan` resolves it, `step_camera` reads `at(out_t)` into `FramePose.clip_mix`.
- `src-tauri/src/export/pipeline/frame_loop.rs` - blends `Pipes::clip_held` into the frame through `screen_mix::blend_into` when the pose carries a mix and the latch matches `prev_clip`.
