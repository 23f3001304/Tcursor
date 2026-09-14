# src-tauri/src/export/cursor/cursorpreview.rs

Editor-preview cursor commands. Expose the export's selected cursor pack and the cursor-type track to the frontend so the editor's canvas preview draws the **same** cursor the export renders (gated by cursor style), instead of a generic arrow. Sprites are decoded/cropped/dark-inverted exactly like the export.

## CursorSpriteDto

```rust
#[derive(serde::Serialize)]
pub struct CursorSpriteDto { pub kind: CursorType, pub url: String, pub hot: [f32; 2], pub canvas_h: u32 }
```

One cursor sprite for the canvas preview. `kind` serializes to the lowercase cursor-type name (`arrow`, `hand`, `resize_ns`, …) - the frontend keys its sprite map by it. `url` is a `data:image/png;base64,…` of the cropped (and, for a dark theme, RGB-inverted) sprite. `hot` is the hotspot as a 0..1 fraction of the cropped sprite. `canvas_h` is the original PNG height, so all shapes scale on one basis (the same the export uses).

## CursorPackDto

```rust
#[derive(serde::Serialize)]
pub struct CursorPackDto {
    pub sprites: Vec<CursorSpriteDto>,
    pub busy_frames: Vec<CursorSpriteDto>,
    pub busy: Option<BusySpec>,
}
```

The recording's selected pack, ready for the canvas preview: one sprite per kind, the pack's explicit busy frames (empty unless it ships `busy_NN.png`), and its declared busy animation.

`busy` + `busy_frames` are what let the preview run the SAME `busy_pose` the export does, so a paused preview shows exactly the frame the export would write for that instant. `cursor_sprites` returning a struct rather than a bare `Vec` is what makes room for them.

### CursorPackDto::material

```rust
pub material: Option<String>,
```

The selected pack's `material` (`"glass"`, else `None`), carried to the canvas preview alongside the sprites.

The preview draws a glass pack's sprite at the same reduced alpha the export blits it at, and cross-fades its states the same way, so the LIVE canvas approximates the paused exact frame the backend renders into the same pixels. It cannot refract - see the deliberate differences in `docs/api/src/editor/stage/cursorGlass.md`.

## cursor_sprites

```rust
#[tauri::command]
pub fn cursor_sprites(folder: String) -> Result<CursorPackDto, String>
```

The recording's selected cursor pack (embedded, bundled, or imported - see `export/cursor/pack.rs`) as PNG data URLs, decoded like the export (crop to alpha, hotspot re-based, RGB-inverted for a dark theme). The preview draws these when the recording's cursor style is Enhanced. Errors only if the Arrow fallback fails to decode.

*Not to be confused with the pack GRID's tiles* (`CursorPanel`), which load raw pack PNGs straight off disk through the asset protocol: this command is the one selected pack, decoded to match the export exactly, and it shells out to ffmpeg once per sprite.

## sprite_dto

```rust
fn sprite_dto(kind: CursorType, png: &[u8], hot: (f32, f32), dark: bool) -> Option<CursorSpriteDto>
```

Decode one pack PNG the way the export does, then re-encode it as a data URL for the canvas. Extracted so the kind sprites and the busy frames go through one path; explicit frames get the busy sprite's own centered hotspot, matching `cursorset::prep`.

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

## CapturedCursorDto

```rust
#[derive(serde::Serialize)]
pub struct CapturedCursorDto { pub id: u32, pub w: u32, pub h: u32, pub hx: u32, pub hy: u32, pub url: String }
```

One captured OS-cursor bitmap for the canvas preview: its layer id, pixel size, hotspot in pixels, and the recorded PNG as a `data:image/png;base64,...` URL.

*Why the PNG is passed through untouched*, unlike `CursorSpriteDto` (which is cropped and dark-inverted to match the export): this IS the real cursor. Cropping would move the hotspot, and inverting it would be exactly the kind of idealization "System" promises not to do.

## CursorLayerDto

```rust
#[derive(serde::Serialize)]
pub struct CursorLayerDto {
    pub cursors: Vec<CapturedCursorDto>,
    pub track: Vec<(u32, u32)>,
    pub src_w: u32,
    pub src_h: u32,
}
```

The whole captured cursor layer. `track` is `(t, id)` in **output** ms - shifted the same way `cursor_kinds` shifts the shape track - so the preview can look both up against the one playhead.

`src_w`/`src_h` are the recorded video's own pixel size. The cursor bitmaps are in SOURCE pixels, so the preview needs it to size them relative to the screen content exactly as the export's `captured::content_scale` does - and it cannot get it from the `<video>` element, which is playing a downscaled proxy. `0` means the probe failed; the preview then falls back to the panel-only scale.

## cursor_layer

```rust
#[tauri::command]
pub fn cursor_layer(folder: String) -> Option<CursorLayerDto>
```

The recording's captured OS-cursor layer, so the editor preview composites the REAL cursor for the "System" style instead of the generic arrow.

**Synchronous**, like `cursor_sprites` and unlike `cursor_kinds`: a small JSON read, a handful of cursor-sized PNGs, and one `probe_dims` - no warm renderer, and `cursor_sprites` already shells out per call through `decode_sprite`, so this is the same weight class. The probe runs only AFTER the layer resolves, so a legacy project (no layer) pays nothing for it. It is deliberately the same `probe_dims(&paths.video())` `FrameRenderer::new` derives its `sw`/`sh` from, so the preview and the export scale the captured cursor from the identical number.

### Inputs

- `folder: String` - absolute project path.

### Returns

`Option<CursorLayerDto>` - `None` for a recording made before the cursor layer existed, and for an unreadable/corrupt one. `Option` rather than `Result`: a missing cursor layer is a normal state (every legacy project), not an error the UI should surface. Entries whose PNG cannot be read are dropped individually.

## output_offset

```rust
fn output_offset(paths: &ProjectPaths) -> i64
```

Event time -> output time in ms: `events_ms - frames[0]`, read straight from `sync.json`.

*Why read the sync log directly instead of going through the warm renderer* the way `cursor_kinds` does: `build_timeline` prefers `sync.json` whenever it has frames, and `frames[0]` IS the `video_start` it hands the renderer - so this produces the identical shift while keeping `cursor_layer` a plain file read. Returns 0 for a recording with no usable sync log, which is what `build_timeline`'s own synthesized timeline uses (`events_ms: 0`).

### Behaviors

- `the_layer_track_is_shifted_onto_the_output_clock` - events starting at 900ms with a first video frame at 1000ms gives -100; a missing `sync.json` gives 0.
- `a_recording_with_no_layer_has_no_captured_cursor` - an empty folder yields `None` rather than an error.

**Dark-theme invert is gated on `pack::theme_inverts`** (2026-09-13): only the embedded default set is flipped; a bundled or imported pack's data URLs carry its real colours, exactly as the export draws them.

