# src-tauri/src/export/pipeline/ffio.rs

FFmpeg and ffprobe spawn helpers and bundled-image decode/crop utilities. All public functions here are pure I/O adapters over the bundled ffmpeg/ffprobe binary located by `process::proc::ffcmd`; they contain no export logic or state. The continuous raw-BGRA-stream decoder (`RawDecoder`) lives in the sibling `ffio_decoder.rs` (re-exported here as `ffio::RawDecoder`, so every existing import path is unchanged) - see `ffio_decoder.md`.

## probe_dims

```rust
pub fn probe_dims(video: &Path) -> Result<(u32, u32)>
```

Queries a video's pixel dimensions via ffprobe.

### Inputs

- `video: &Path` - path to the video file. *Why:* passed as the ffprobe input argument.*

### Returns

`Result<(u32, u32)>` - `(width, height)` in pixels. Errors if ffprobe cannot be spawned or the CSV output cannot be parsed as two positive integers.

### Implementation

1. Spawn `ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0 <video>`, capturing stdout.
2. Take the first non-empty line, split on `,`, parse each part as `u32`.
3. Return `Err` with the raw line when either parse fails.

## probe_duration

```rust
pub fn probe_duration(video: &Path) -> Result<f64>
```

Queries a video's container duration in seconds. Returns `0.0` if the duration field is absent or unparseable (non-fatal sentinel; callers must handle zero).

### Inputs

- `video: &Path` - path to the video file. *Why:* duration is used for timeline calculations and frame-count estimation.*

### Returns

`Result<f64>` - duration in seconds; `0.0` on parse failure. Errors only if ffprobe cannot be spawned.

### Implementation

1. Spawn `ffprobe -v error -show_entries format=duration -of default=nk=1:nw=1 <video>`.
2. Take the first non-empty line; `parse::<f64>().unwrap_or(0.0)` treats "N/A" or empty output as zero.

## decode_image

```rust
pub fn decode_image(image: &[u8], w: u32, h: u32) -> Result<Vec<u8>>
```

Decodes any ffmpeg-readable image format to a `w * h * 4` BGRA buffer, STRETCHED to the exact output size. Used for the legacy bundled background (`assets/backgrounds/bg.jpg`). *Why a plain stretch:* every project saved before the wallpaper library renders through this call, and its exact look - aspect distortion and all - is what those projects expect. The library's own images take the cover-fitting `decode_image_cover` below instead.

### Inputs

- `image: &[u8]` - raw image bytes (PNG, JPEG, WebP, etc.). *Why bytes not path:* the wallpaper is embedded in the binary; writing to a temp file lets ffmpeg decode any format without adding a Rust image decoder dependency.*
- `w: u32`, `h: u32` - desired output dimensions. *Why:* the background must exactly match the output frame size.*

### Returns

`Result<Vec<u8>>` - BGRA buffer of exactly `w * h * 4` bytes. Errors if the input cannot be staged, ffmpeg cannot be spawned, or output length mismatches.

### Implementation

1. Stage `image` via `StagedInput::new` - a temp file at a path unique to this call. *Why temp file:* ffmpeg cannot read piped data for all image formats.*
2. Spawn `ffmpeg -v error -i <staged> -frames:v 1 -f rawvideo -pix_fmt bgra -vf scale=w:h -`, capturing stdout.
3. Assert `stdout.len() == w * h * 4`; error if not. The staged file is deleted when the guard drops, including on the early `?` return.

Steps 1-3 live in the private `decode_with`, which both this and `decode_image_cover` call with their own `-vf` filter - the only thing that differs between them.

### Behaviors worth knowing

- `decode_image_is_immune_to_a_clobber_of_the_legacy_shared_path` - a decode's result depends only on the bytes it was handed, even while an unrelated writer hammers `$TEMP/cursorzoom_bg_src` (the path every caller used to share) with a different image.

## decode_image_cover

```rust
pub fn decode_image_cover(image: &[u8], w: u32, h: u32) -> Result<Vec<u8>>
```

`decode_image` with a COVER fit: the image is scaled until it fills `w` x `h` with its aspect ratio intact, then centre-cropped. Used for the bundled wallpaper library (`settings::wallpapers`).

### Inputs

Identical to `decode_image` - raw image bytes plus the output frame's dimensions.

### Returns

`Result<Vec<u8>>` - BGRA buffer of exactly `w * h * 4` bytes, same contract as `decode_image`.

### Implementation

Calls `decode_with` with `-vf scale=w:h:force_original_aspect_ratio=increase,crop=w:h`. *Why cover and not fit:* the wallpapers are 16:9 art; letterboxing them into a 9:16 or 1:1 export would put bars behind the screen panel, and stretching them would visibly distort the ribbons. Cropping the overflow is the only option that keeps the frame filled and the art undistorted. At a 16:9 output the `crop` is a no-op and the result is the same plain scale `decode_image` would have produced.

### Behaviors worth knowing

- `cover_fit_crops_the_overflow_while_the_legacy_decode_still_stretches` - a 4x2 image whose outer columns are green decodes into a 2x2 square that is entirely the red centre, while `decode_image` on the same input folds the green edges in. Skipped (not failed) when no ffmpeg is on PATH.

## decode_file_cover

```rust
pub fn decode_file_cover(file: &Path, w: u32, h: u32) -> Result<Vec<u8>>
```

`decode_image_cover` for a file already ON DISK: the user's imported background (`settings::bg_asset`). Same filter, same BGRA contract, same cover framing - so an imported still is framed exactly like a bundled wallpaper.

*Why a second entry point rather than reading the bytes and reusing `decode_image_cover`:* that path stages its input through a temp file, so a multi-MB import would be read into memory and written back out just to hand ffmpeg a path it could have had directly. A VIDEO background is decoded here too, for its FIRST frame (what the Rust preview commands show and what the export falls back to if its stream dies) - and those files can be gigabytes.

An `Err` here is the ordinary case for a deleted or unreadable asset, and is what makes `export::scene::background::build` fall back to the wallpaper.

### Behaviors worth knowing

- `the_file_input_decode_frames_exactly_like_the_in_memory_one` - the same fixture decoded both ways is byte-identical, and a missing file is an `Err`. Skipped (not failed) when no ffmpeg is on PATH.

## StagedInput

```rust
pub(crate) struct StagedInput(std::path::PathBuf); // private to ffio.rs
```

An input image written to disk for ffmpeg to read, at `$TEMP/cursorzoom_img_<pid>_<n>` (`n` from a process-wide `AtomicU64`), removed by `Drop`.

### Why it exists

`decode_image` used to stage every image at one fixed `$TEMP/cursorzoom_bg_src`, and it has two unrelated callers that run on different threads: `background::build` (the mesh wallpaper, on a cold `FrameRenderer::new` for a preview *or* an export) and `decode_cursor` (every cursor-pack sprite, via the `cursor_sprites` command and `cursorset::prep`). The editor fires both within milliseconds of opening a project, so a sprite decode could overwrite the wallpaper's staged bytes in the window between the write and ffmpeg's read - the background then decoded from `resize_ns.png`, scaled to the full frame. That is what `preview_bg` returned to the editor stage, and what `composite_at` drew under every panel. A per-call path (unique within the process by the counter, across processes by the pid) makes the collision impossible rather than unlikely.

Dropping the guard also deletes the file on the error path, which the old inline `remove_file` (placed after a `?`) skipped.

Crate-visible since 2026-09-14: `export::preview::jpeg_encode` stages a composited BGRA frame through it the same way `decode_with` stages an image.

## png_dims

```rust
pub fn png_dims(png: &[u8]) -> Option<(u32, u32)>
```

Reads a PNG's pixel dimensions directly from its IHDR chunk without spawning any process. Returns `None` if the bytes are not a valid PNG or are truncated.

### Inputs

- `png: &[u8]` - raw PNG bytes. *Why:* cursor sprites are embedded bytes; reading dimensions from the header avoids spawning ffprobe for a trivial 24-byte check.*

### Returns

`Option<(u32, u32)>` - `(width, height)` from IHDR bytes 16-19 and 20-23 (big-endian u32). `None` if the PNG signature check fails, the buffer is shorter than 24 bytes, or either dimension is zero.

## crop_to_alpha

```rust
pub fn crop_to_alpha(bgra: &[u8], w: u32, h: u32) -> Option<(Vec<u8>, u32, u32, u32, u32)>
```

Trims fully-transparent border rows and columns from a BGRA image. Alpha <= 16 is treated as empty, allowing near-transparent antialiasing fringe to be included in the crop.

### Inputs

- `bgra: &[u8]` - BGRA pixel buffer. *Why:* cursor sprites have transparent padding that wastes atlas space and displaces hotspot calculations.*
- `w: u32`, `h: u32` - source image dimensions. *Why:* required to index the flat buffer.*

### Returns

`Option<(Vec<u8>, u32, u32, u32, u32)>` - `(cropped_bgra, cw, ch, left, top)` where `(left, top)` is the crop-box origin in the source image. `None` if the entire image is transparent or dimensions are zero.

### Implementation

1. Walk every pixel; for alpha > 16, update min/max for x (`l, r`) and y (`t, b`).
2. If `r < l` (no opaque pixel found), return `None`.
3. Compute `cw = r - l + 1`, `ch = b - t + 1`; copy each row of the crop box into a new buffer row by row.

### Behaviors worth knowing

- `crop_trims_to_content_and_reports_origin` - a 4x4 image with one opaque pixel at `(x=1, y=2)` returns a 1x1 buffer, `(cw=1, ch=1, left=1, top=2)`. A fully-transparent 4x4 image returns `None`. A zero-dimension input returns `None` without panicking.

## decode_cursor

```rust
pub fn decode_cursor(png: &[u8], hot: (f32, f32)) -> Option<(Vec<u8>, u32, u32, (f32, f32), u32)>
```

Decodes a cursor PNG sprite to a content-tight BGRA buffer and re-bases the hotspot fraction to the cropped content box.

### Inputs

- `png: &[u8]` - raw PNG bytes of the cursor sprite. *Why:* cursor sprites are embedded in `cursor.json` as base64; this decodes one sprite.*
- `hot: (f32, f32)` - hotspot as fractions of the full canvas (`0..1, 0..1`). *Why:* the hotspot must be re-expressed relative to the cropped content box for correct cursor positioning in the output frame.*

### Returns

`Option<(Vec<u8>, u32, u32, (f32, f32), u32)>` - `(bgra, content_w, content_h, content_hotspot, native_canvas_h)`. `native_canvas_h` lets a sprite packer scale all cursors uniformly by their canvas height, preserving relative sizes across cursor types. `None` on any failure in the chain.

### Implementation

1. `png_dims(png)?` -> native canvas `(nw, nh)`.
2. `decode_image(png, nw, nh).ok()?` -> full BGRA at native size.
3. `crop_to_alpha(&src, nw, nh)?` -> `(bgra, w, h, l, t)`.
4. Re-base hotspot: `hx = (hot.0 * nw - l) / w`, `hy = (hot.1 * nh - t) / h`. *Why:* the original hotspot is in canvas-fraction coordinates; converting to content-box-fraction coordinates keeps the cursor tip at the correct pixel after the sprite is cropped.*

## probe_frame_count

```rust
pub fn probe_frame_count(video: &Path) -> Result<u64>
```

Returns the total video frame count, preferring ffprobe's `nb_frames` field and falling back to `avg_frame_rate * duration` when `nb_frames` is absent.

### Inputs

- `video: &Path` - path to the video file. *Why:* the exporter needs a frame count to size the render loop and progress reporting.*

### Returns

`Result<u64>` - total frames. Errors if neither `nb_frames` nor the rate-times-duration calculation yields a usable value.

### Implementation

1. Spawn `ffprobe -v error -select_streams v:0 -show_entries stream=nb_frames,avg_frame_rate,duration -of default=nw=1 <video>`.
2. Parse each `key=value` line: collect `nb` (positive u64), `rate` (from `num/den` fraction), `dur` (f64).
3. Return `nb` if non-zero. Else compute `(rate * dur).round() as u64`. Error if neither is computable.
