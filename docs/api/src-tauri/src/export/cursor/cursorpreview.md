# src-tauri/src/export/cursor/cursorpreview.rs

Editor-preview cursor commands. Expose the export's selected cursor pack and the cursor-type track to the frontend so the editor's canvas preview draws the **same** cursor the export renders (gated by cursor style), instead of a generic arrow. Sprites are decoded/cropped/dark-inverted exactly like the export.

## CursorSpriteDto

```rust
#[derive(serde::Serialize)]
pub struct CursorSpriteDto { pub kind: CursorType, pub url: String, pub hot: [f32; 2], pub canvas_h: u32 }
```

One cursor sprite for the canvas preview. `kind` serializes to the lowercase cursor-type name (`arrow`, `hand`, `resize_ns`, …) — the frontend keys its sprite map by it. `url` is a `data:image/png;base64,…` of the cropped (and, for a dark theme, RGB-inverted) sprite. `hot` is the hotspot as a 0..1 fraction of the cropped sprite. `canvas_h` is the original PNG height, so all shapes scale on one basis (the same the export uses).

## cursor_sprites

```rust
#[tauri::command]
pub fn cursor_sprites(folder: String) -> Result<Vec<CursorSpriteDto>, String>
```

The recording's selected cursor pack as PNG data URLs, decoded like the export.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* both the dark-theme inversion (`settings.ui.theme`) and the selected pack (`settings.cursor.pack`) come from the recording's settings snapshot (`load_or_seed`), so the sprites match that recording's export exactly.

### Returns

`Result<Vec<CursorSpriteDto>, String>` - one entry per resolved sprite (non-arrow decode failures are skipped). Errors only if the `arrow` fallback fails to decode.

### Implementation

1. `settings = load_or_seed(folder).settings`; `dark = resolve_dark(settings.ui.theme)`.
2. For each `(kind, png, hot)` in `pack::sprite_sources(&settings.cursor.pack)` (built-in bytes for `"default"`, else an imported pack falling back per-kind - see `export/cursor/pack.rs`): `cursordraw::decode_sprite` (ffmpeg PNG→BGRA, crop to alpha, hotspot re-based) → invert RGB if `dark` → `preview::png_encode` → base64 data URL. (~2 ffmpeg spawns per sprite; one-shot per pack change.)

## CursorKindSample

```rust
#[derive(serde::Serialize)]
pub struct CursorKindSample { pub t: u32, pub kind: CursorType }
```

One cursor-shape change at output time `t` (ms); `kind` serializes to the lowercase type name.

## cursor_kinds

```rust
#[tauri::command]
pub async fn cursor_kinds(folder: String, app: tauri::AppHandle) -> Result<Vec<CursorKindSample>, String>
```

The cursor-type track in **output** time, so the preview picks the right sprite as the shape changes.

**Off the main thread (sweep-2 Task 1).** `async fn` + `spawn_blocking`, the pattern `preview_frame` documents (`export/preview/mod.md`). Its own body is a small file read plus a map, but it goes through `with_warm`, whose cold path is `FrameRenderer::new` (event-log decode, ffprobe/ffmpeg subprocesses, wgpu init, cursor-pack prep) - and this is one of six `with_warm` commands the editor fires on the same mount tick. `tauri::State<'_, PreviewSession>` cannot cross into `spawn_blocking`, so the command takes `app: tauri::AppHandle` and re-derives the managed state inside the closure; Tauri injects `AppHandle`, so the JS call is unchanged.

### Inputs (what, and why it is needed)

- `folder: String` - absolute project path. *Why:* identifies the recording (`cursor.json`).
- `app: tauri::AppHandle` - resolves the warm renderer cache (`PreviewSession`) inside the blocking closure. *Why:* it provides `events_ms` + `video_start` to map event time to output time.

### Returns

`Result<Vec<CursorKindSample>, String>` - the `cursor.json` samples whose output time is >= 0, ascending.

### Implementation

Inside `with_warm`, load `CursorTrack` from `paths.cursor()` and map each `(et, kind)` to output ms with `et + events_ms - video_start` (identical to `click_track`), dropping pre-start samples.
