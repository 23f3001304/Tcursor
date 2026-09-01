# src-tauri/src/export/preview/preview_layouts.rs

The one Tauri command that returns **all five** layout presets' panel rects at once, so the editor preview can resolve and cross-fade between layouts itself (mirroring `LayoutTrack::scene_at`) instead of only ever showing the single static layout `preview_layout` returns. Split out of `preview_track.rs` to keep both files inside the 200-line budget; it reuses the same `with_warm` renderer cache (`session.rs`).

## PanelRectDto

```rust
#[derive(serde::Serialize)]
pub struct PanelRectDto { pub rect: [f32; 4], pub radius: f32, pub alpha: f32, pub ring_px: f32, pub ring_color: [u8; 3] }
```

One panel's framing, normalized for the frontend: `rect` is `[x, y, w, h]` as fractions of the output (x/w by output width, y/h by output height), `radius` and `ring_px` are fractions of the output **width** (the same basis `preview_layout` uses, so a rounded corner stays circular), `alpha` is the panel's 0..1 opacity in that preset. `ring_color` is only meaningful when `ring_px > 0` - the screen panel never has a ring and reports `0` / `[0, 0, 0]`.

## LayoutPresetDto

```rust
#[derive(serde::Serialize)]
pub struct LayoutPresetDto { pub screen: PanelRectDto, pub cam: PanelRectDto }
```

One preset's two panels. Unlike `PreviewLayout`'s `Option<[f32; 9]>` camera, `cam` is always present here - a preset where the camera is hidden reports it with `alpha: 0`, which is exactly what a cross-fade needs to interpolate towards.

## LayoutPresets

```rust
#[derive(serde::Serialize)]
pub struct LayoutPresets {
    pub screen: LayoutPresetDto, pub camera: LayoutPresetDto, pub presenter: LayoutPresetDto,
    pub screen_only: LayoutPresetDto, pub camera_only: LayoutPresetDto,
}
```

All five `LayoutId` presets in one payload, named to match the enum (`Screen`, `Camera`, `Presenter`, `ScreenOnly`, `CameraOnly`). No `serde(rename_all)`, so the wire keys are the snake_case field names - which is what the TS side declares (`LayoutPresetName = "screen" | "camera" | "presenter" | "screen_only" | "camera_only"` in `src/lib/ipc.ts`).

## panel_dto

```rust
fn panel_dto(p: &Panel, ow: f32, oh: f32) -> PanelRectDto
```

Normalizes one resolved `Panel` (output pixels) into a `PanelRectDto` (fractions). `rect.x`/`rect.w` divide by `ow`, `rect.y`/`rect.h` by `oh`, and `radius`/`ring_px` by `ow`.

## preview_layouts

```rust
#[tauri::command]
pub async fn preview_layouts(folder: String, app: tauri::AppHandle) -> Result<LayoutPresets, String>
```

Returns every layout preset's panel rects + alpha for one recording.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`, the pattern `preview_frame` documents (`mod.md`). The body itself is cheap (`resolve_layout` five times, pure scene math on the warm renderer), but it goes through `with_warm`, whose cold path runs `FrameRenderer::new` - event-log decode, up to three `ffprobe`/`ffmpeg` subprocess spawns, two wgpu pipeline builds, cursor-pack prep. This is one of six `with_warm` commands `useEditorData` fires on the same editor-mount tick; as sync commands they all ran on the UI thread, so whichever one lost the race still waited out the winner's cold build with the window frozen. `tauri::State<'_, PreviewSession>` cannot cross into `spawn_blocking` (its lifetime is not `'static`), so the command takes `app: tauri::AppHandle` and re-derives the managed state inside the closure; `AppHandle` is injected by Tauri, so the JS call (`previewLayouts(folder)`) is unchanged.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording; the presets depend on its source dimensions and the doc's aspect.
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) inside the blocking closure. *Why:* the presets come from the same `FrameRenderer` the export composites with, so preview and export agree by construction.

### Returns

`Result<LayoutPresets, String>` - the five presets. Errors (as a string) if the renderer cannot be built; a `spawn_blocking` join failure maps to the same shape.

### Implementation

Inside `with_warm`, take `(ow, oh)` from the cached `RenderMeta`, then call `renderer.resolve_layout(id)` for each of the five `LayoutId`s and run both of that scene's panels through `panel_dto`.
