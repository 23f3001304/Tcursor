# src-tauri/src/export/preview/preview_layouts.rs

The one Tauri command that returns **all five** layout presets' panel rects at once, so the editor preview can resolve and cross-fade between layouts itself (mirroring `LayoutTrack::scene_at`) instead of only ever showing the single static layout `preview_layout` returns. Split out of `preview_track.rs` to keep both files inside the 200-line budget; it reuses the same `with_warm` renderer cache (`session.rs`).

**T34 L2: also returns each DOC SEGMENT's own resolved rects.** Alongside the 5 presets, the response carries `segs: Vec<SegRectDto>` - one entry per `EditDoc.layout` segment, resolved from that segment's arrangement-or-preset via `FrameRenderer::resolve_seg` (`render/accessors.md`), the exact function the export's `LayoutTrack` uses per segment. This is how a segment carrying a T34 `Arrangement` previews its real posed panels instead of its provenance preset's - see `src/editor/timeline/layoutTrack.md`'s `layoutAt` for the TS side that consumes it.

## PanelRectDto

```rust
#[derive(serde::Serialize)]
pub struct PanelRectDto { pub rect: [f32; 4], pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }
```

One panel's framing, normalized for the frontend: `rect` is `[x, y, w, h]` as fractions of the output (x/w by output width, y/h by output height), `radius` and `ring_px` are fractions of the output **width** (the same basis `preview_layout` uses, so a rounded corner stays circular), `alpha` is the panel's 0..1 opacity in that preset. `ring_color` is only meaningful when `ring_px > 0` - the screen panel never has a ring and reports `0` / `[0, 0, 0]`.

## LayoutPresetDto

```rust
#[derive(serde::Serialize)]
pub struct LayoutPresetDto { pub screen: PanelRectDto, pub cam: PanelRectDto,
    pub arrangement: crate::edit::model::Arrangement }
```

One preset's two panels. Unlike `PreviewLayout`'s `Option<[f32; 9]>` camera, `cam` is always present here - a preset where the camera is hidden reports it with `alpha: 0`, which is exactly what a cross-fade needs to interpolate towards.

`arrangement` is the SAME preset expressed as POSES (`scene::arrangement::arrangement_of_preset`) - what the editor feeds a `SetArrangement` op to turn this preset into a directly manipulable arrangement, so the preset-to-poses conversion has exactly one definition and it lives in Rust. A preset that hides a panel reports `null` for it, matching the `Arrangement` rule.

**Why it rides on this response instead of a dedicated `arrangement_of` command:** the derivation is a pure function of the very `Scene` this call already resolves, and the editor already fetches all five presets on mount. A separate command would re-resolve the same scenes behind another `with_warm` round trip on the same tick - a strictly larger surface for no new information.

## SegRectDto

```rust
#[derive(serde::Serialize)]
pub struct SegRectDto { pub id: String, pub screen: Option<PanelRectDto>, pub cam: Option<PanelRectDto> }
```

One `EditDoc.layout` segment's resolved panels, by id. `None` on a field means "this segment doesn't override that panel - fall back to its `layout` preset", which is BOTH fields on a segment with no `arrangement` at all; a segment WITH one always resolves both fields (`Some`), never a mix, since `resolve_arrangement` always returns a full scene - a panel the arrangement hides still gets a real rect, just `alpha: 0` (L1's design, so a cross-dissolve into/out of it slides rather than pops).

## LayoutPresets

```rust
#[derive(serde::Serialize)]
pub struct LayoutPresets {
    pub screen: LayoutPresetDto, pub camera: LayoutPresetDto, pub presenter: LayoutPresetDto,
    pub screen_only: LayoutPresetDto, pub camera_only: LayoutPresetDto,
    pub segs: Vec<SegRectDto>,
}
```

All five `LayoutId` presets in one payload, named to match the enum (`Screen`, `Camera`, `Presenter`, `ScreenOnly`, `CameraOnly`). No `serde(rename_all)`, so the wire keys are the snake_case field names - which is what the TS side declares (`LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only"` in `src/lib/ipc.ts`).

`segs: Vec<SegRectDto>` (T34 L2) - one entry per `EditDoc.layout` segment, IN DOC ORDER, not just posed ones: a lookup by id is then a single flat scan with no special-casing "this segment was never in the list" vs. "present but plain".

## panel_dto

```rust
fn panel_dto(p: &Panel, ow: f32, oh: f32) -> PanelRectDto
```

Normalizes one resolved `Panel` (output pixels) into a `PanelRectDto` (fractions). `rect.x`/`rect.w` divide by `ow`, `rect.y`/`rect.h` by `oh`, and `radius`/`ring_px` by `ow`.

## seg_rect_dto

```rust
fn seg_rect_dto(seg: &crate::edit::model::LayoutSeg, scene: impl FnOnce() -> crate::export::scene::Scene,
    ow: f32, oh: f32) -> SegRectDto
```

Builds one `SegRectDto`. Pure and independently testable (no warm renderer needed) - `scene` is an `FnOnce` closure invoked AT MOST ONCE, only when `seg.arrangement.is_some()`, so a doc full of plain-preset segments costs nothing extra: `preview_layouts` passes `|| c.renderer.resolve_seg(seg)` and this function only calls it when there is an arrangement to resolve.

### Implementation

- `seg.arrangement.is_none()` -> `SegRectDto { id, screen: None, cam: None }` immediately, without calling `scene`.
- `seg.arrangement.is_some()` -> calls `scene()` once, then runs both of the resulting `Scene`'s panels through `panel_dto` into `Some(..)`.

Pinned by `preview_layouts_tests.rs`: a plain segment never resolves a scene at all; an arrangement segment's DTO rects match `resolve_seg_scene`'s output run through `panel_dto` directly (proving no second conversion path); a hidden posed panel still resolves to `Some(..)` at `alpha: 0`, not `None`; the `id` always round-trips onto the DTO.

## preview_layouts

```rust
#[tauri::command]
pub async fn preview_layouts(folder: String, app: tauri::AppHandle) -> Result<LayoutPresets, String>
```

Returns every layout preset's panel rects + alpha, plus each doc segment's own resolved rects (`segs`), for one recording.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`, the pattern `preview_frame` documents (`mod.md`). The body itself is cheap (`resolve_layout` five times plus `resolve_seg` per arrangement segment, pure scene math on the warm renderer), but it goes through `with_warm`, whose cold path runs `FrameRenderer::new` - event-log decode, up to three `ffprobe`/`ffmpeg` subprocess spawns, two wgpu pipeline builds, cursor-pack prep. This is one of six `with_warm` commands `useEditorData` fires on the same editor-mount tick; as sync commands they all ran on the UI thread, so whichever one lost the race still waited out the winner's cold build with the window frozen. `tauri::State<'_, PreviewSession>` cannot cross into `spawn_blocking` (its lifetime is not `'static`), so the command takes `app: tauri::AppHandle` and re-derives the managed state inside the closure; `AppHandle` is injected by Tauri, so the JS call (`previewLayouts(folder)`) is unchanged.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; the presets depend on its source dimensions and the doc's aspect, and `segs` depends on the doc's own `layout` segments.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) inside the blocking closure. *Why:* the presets and segments come from the same `FrameRenderer` the export composites with, so preview and export agree by construction.

### Returns

`Result<LayoutPresets, String>` - the five presets plus `segs`. Errors (as a string) if the renderer cannot be built; a `spawn_blocking` join failure maps to the same shape.

### Implementation

Inside `with_warm`, take `(ow, oh)` from the cached `RenderMeta`, then call `renderer.resolve_layout(id)` for each of the five `LayoutId`s, run both of that scene's panels through `panel_dto`, and run the scene itself through `scene::arrangement::arrangement_of_preset` for the pose form.

Then read the doc via `edit::seed::load_or_seed(paths)` - self-locking (see its own doc comment), safe to call here with no extra locking of its own even though `with_warm`'s `build`/`reuse` already called it once this same warm-up, since once the doc is current (the common, warm-cache case) it is a pure read. Map `doc.layout` through `seg_rect_dto(seg, || c.renderer.resolve_seg(seg), ow, oh)` into `segs`.
