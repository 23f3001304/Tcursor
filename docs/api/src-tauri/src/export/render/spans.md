# src-tauri/src/export/render/spans.rs

The take's SOURCE SPANS: which part of the recorded canvas the screen panel shows over time, and what happens at the instant that changes.

A mid-take display switch (`session::record::switch_display`) keeps ONE encoder canvas - the first display's size - and fits every later frame into it (`session::record::frame_fit::letterbox`), so `video.mp4` carries baked black bars from the switch onwards. This module undoes that at render time. The take becomes a list of spans, each `[start_ms (output clock), src_rect (source px)]`, and the screen panel shows exactly `src`: the whole canvas before the first switch, the fitted rect of the switched-to display after it. The panel therefore takes each span's own aspect - a 16:10 display gets a 16:10 panel with the project background beside it, corners and shadow intact - which is why every span gets its own `LayoutTrack`: `scene::resolve` shapes the screen panel from the source size it is handed.

The switch itself eases rather than cutting: over `SWITCH_MS` the panel blends from the old span's resolved scene to the new one's through `Scene::lerp` - the very machinery a layout transition runs on, not a second animator - while the two PICTURES cross-dissolve (`SpanMix`, applied by `render::screen_mix` on the decoded nv12 frame before the compositor ever sees it).

A take with no switch is exactly one full-canvas span: every consumer degenerates to its pre-span behavior, which is what makes such a take render byte-identically to before.

## SWITCH_MS

```rust
pub const SWITCH_MS: u32 = 350;
```

Display-switch transition length (ms) - the same 350 a layout segment gets by default (`render_edit`'s `TRANSITION_MS`), on the same `Easing::Smooth` curve, so a switch feels like the rest of the editor's motion rather than a second idiom.

## SourceSpan

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceSpan { pub start_ms: u32, pub src: RectF }
```

One span: from `start_ms` on the OUTPUT clock, the screen panel shows `src` of the recorded canvas. The first span always starts at 0 with the whole canvas.

- `start_ms: u32` - output time. *Why output, not recording:* every region list in the renderer lives on the output clock after `edit::remap_doc`, so a span inside a cut or a speed span moves with everything else.
- `src: RectF` - source pixels. *Why the exact `letterbox` rect rather than a re-derived one:* the crop has to land on the fitted picture to the pixel, and `frame_fit::letterbox` is the function that placed it.

## SpanMix

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpanMix { pub prev_src: RectF, pub alpha: f32, pub prev_span: usize, pub hold_ms: u32 }
```

A display switch in flight at one instant.

- `prev_src: RectF` - the rect the outgoing picture occupies in the held frame. *Why:* `screen_mix` resamples that rect into the NEW span's rect, so both pictures then ride one crop into the eased panel.
- `alpha: f32` - the eased 0..1 weight of the NEW span's picture (0 at the switch instant, 1 when the transition ends). The same curve the panel rect eases on, so shape and content arrive together.
- `prev_span: usize` - which span the held frame must be from. *Why:* a cut can skip the frame the caller was told to latch; comparing indices drops the dissolve instead of blending a stale picture.
- `hold_ms: u32` - the output tick whose screen frame to blend FROM, so the exporter (which latches it as frames stream past) and the one-shot preview (which seek-decodes it) use the same picture.

## hold_tick

```rust
pub fn hold_tick(start_ms: u32, fps: u64) -> u32
```

The output tick whose decoded screen frame a switch at `start_ms` dissolves from: the LAST output frame before the switch. Derived arithmetically (`j = (start * fps - 1) / 1000`, then `j * 1000 / fps`) rather than observed, so both the streaming exporter and the random-access preview name the same frame with no shared state.

## spans_of

```rust
pub fn spans_of(switches: &[DisplaySwitch], frames: &[u64], canvas: (u32, u32), video_start: u64, map: &TimeMap) -> Vec<SourceSpan>
```

The span table for a take. `switches` are `sync.json`'s entries and `frames` its frame times (both on the recording clock, `frames` ascending); `canvas` is the video's own size (`probe_dims`, already evened).

### Behaviors worth knowing

- No switches: exactly one full-canvas span. The pin every existing recording rests on.
- A span opens on the first `frames` entry at or after the switch stamp, not on the stamp itself: `switch_display` stamps BEFORE the restart, and the replacement capture's first frame lands 50 to 300 ms later. Opened at the stamp, the span would crop and dissolve the OLD display's held frame through the restart gap and the picture would pop to the new one mid-fade (`a_span_opens_on_the_first_frame_after_the_stamp`). No frame after the stamp (a switch right before Stop): the stamp stands.
- Each instant then goes recording clock -> clip clock (`- video_start`) -> output clock (`map.out_of`), exactly like every other recorded region.
- A switch whose geometry was never recorded (`w`/`h` 0 - a log written before `DisplaySwitch` carried them) spans the FULL canvas, so an older project keeps rendering the way it always did rather than cropping to a guess.
- Two switches landing on the same output instant (the same millisecond, or both inside one cut, where `out_of` collapses a range onto a point) leave ONE span carrying the later one's rect - never a zero-length span the transition would divide by. A switch at output 0 replaces span 0's rect instead of adding a span at 0.

## spans_for

```rust
pub fn spans_for(paths: &ProjectPaths, canvas: (u32, u32), video_start: u64, map: &TimeMap) -> Vec<SourceSpan>
```

`spans_of` fed from the recording's own `sync.json` (its `display_switches` and `frames`). A missing, unreadable or switch-free log is a take that never switched: one full-canvas span.

## fitted_rect

```rust
fn fitted_rect(src: (u32, u32), canvas: (u32, u32)) -> RectF
```

Where a `src`-sized capture lands inside the `canvas`-sized encoder frame, as a `RectF`. A thin wrapper over `session::record::frame_fit::letterbox` - deliberately the SAME function the capture's video processor used, so the crop and the fit can never drift apart.

## SpanTrack

```rust
pub struct SpanTrack { spans: Vec<SourceSpan>, tracks: Vec<LayoutTrack>, transition_ms: u32 }
```

The take's spans plus one `LayoutTrack` per span (built at that span's own source size), which together answer "what does this frame look like". A take with no switch has exactly one span and one track, and every method degenerates to that track's own answer.

### Used by

- `src-tauri/src/export/render/render_edit.rs` - `EditState.track`; rebuilt on every edit.
- `src-tauri/src/export/render/step.rs` - `step_camera` asks it for the frame's scene, mix and hold.
- `src-tauri/src/export/scene/layout.rs` - `anchor_frame` re-anchors zoom regions through each frame's own scene (which `frame_at` resolves from it).

## SpanTrack::build

```rust
pub fn build(spans: Vec<SourceSpan>, mut track_for: impl FnMut(u32, u32) -> LayoutTrack) -> Self
```

Build from a span table and a per-span `LayoutTrack` factory, called with that span's own `(sw, sh)` - the size that shapes the screen panel. Taking a factory rather than a built track is what lets `EditState::load` keep its two track flavors (recorded actions vs. edited segments) in one place while this file stays free of appearance settings.

## SpanTrack::spans

```rust
pub fn spans(&self) -> &[SourceSpan]
```

The span table, for `preview_layouts` to hand to the editor.

## SpanTrack::scene_at

```rust
pub fn scene_at(&self, t: u32) -> Scene
```

The scene at `t` with any switch transition already blended in - what every consumer that needs only geometry asks for (zoom anchoring, the preview's per-segment rects). `frame_at` without the frame-latching extras, and pinned by test to agree with it.

## SpanTrack::frame_at

```rust
pub fn frame_at(&self, t: u32, fps: u64) -> (Scene, Option<SpanMix>, Option<usize>)
```

Everything one output frame needs: the blended scene, the display-switch dissolve in flight (if any), and - when this is the last output frame before the NEXT switch - the index of the span whose decoded screen frame the caller must latch for that dissolve.

### Behaviors worth knowing

- Outside a transition the scene is the active span's own track's scene, carrying that span's `src`; `mix` is `None`.
- Inside one the scene is `Scene::lerp(previous span's scene at t, this span's scene at t, eased)`, so the panel morphs between the two aspects while any layout transition underneath keeps running.
- `src` is the NEW span's from the switch instant on (never interpolated) because `screen_mix` pre-blends the outgoing picture INTO that rect.
- Only ONE switch is ever in flight: a second switch inside the first's 350 ms takes over from the (already settled) span before it, exactly as `LayoutTrack` lets a segment's entry win over its predecessor's exit rather than double-blending.
