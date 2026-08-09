# src-tauri/src/win/sys/brand_icon.rs

Dynamic app icon + Windows taskbar progress (Task 39 - "the wave lives"): swaps the main window's icon between the normal brand mark and a REC-lit variant while recording, and drives the taskbar progress bar during export. Both are pure brand flair - graceful no-ops on any failure or off-Windows (mirrors `capture_exclusion.rs`'s `#[cfg(windows)]`/`#[cfg(not(windows))]` split), so a window-manager quirk or a headless run can never turn a decorative touch into a crash or a stuck recording/export.

## ICON_NORMAL

```rust
const ICON_NORMAL: &[u8] = include_bytes!("../../../icons/icon.png");
```

The existing 512x512 RGBA8 app icon, embedded at compile time - unchanged, reused rather than regenerated.

## ICON_REC

```rust
const ICON_REC: &[u8] = include_bytes!("../../../icons/icon-rec.png");
```

A same-size, same-style icon variant, embedded at compile time, generated from `icons/source.svg` with the REC dot enlarged (r=7 -> 11) and given a soft glow - keeping the tile/wave/shadow layers pixel-identical to `ICON_NORMAL` so only the dot itself reads as "lit".

## set_recording

```rust
pub fn set_recording(app: &AppHandle, recording: bool)
```

Swap the main window's icon: the REC-lit variant while `recording`, else the normal mark.

### Inputs

- `app: &AppHandle` - resolves the `"main"` webview window (`get_webview_window("main")`, same lookup `commands::set_capturable` uses).
- `recording: bool` - which icon to show. *Why a bool, not richer state:* the recorder only has two icon-relevant states - paused counts as `true` (still visually "recording"; only Stop clears it) - so a bool is the whole contract.

### Behavior

`#[cfg(windows)]`: decodes the matching bytes via `decode_png` and calls `win.set_icon(img)`, both `?`-free (`let Some(..) = .. else { return }` / `let _ =`) - any failure (missing window, undecodable bytes, OS rejection) is silently swallowed. `#[cfg(not(windows))]`: no-op.

### Used by

- `src-tauri/src/session/record/recorder.rs` (`start_recording`, `stop_recording`) - `true` once a recording session is actually stored in `Running`; `false` once one is actually taken out of it.

## set_export_progress

```rust
pub fn set_export_progress(app: &AppHandle, pct: Option<u8>)
```

Drive the Windows taskbar progress bar from an export's percent-complete.

### Inputs

- `app: &AppHandle` - resolves the main window, same as `set_recording`.
- `pct: Option<u8>` - `Some(p)` shows a normal-state bar at `p`; `None` clears it.

### Behavior

`#[cfg(windows)]`: calls `win.set_progress_bar(progress_state(pct))`, swallowing any failure. `#[cfg(not(windows))]`: no-op.

### Used by

- `src-tauri/src/export/pipeline/run.rs` (`run_export`) - `Some(p)` on each progress tick (mirroring the `"export-progress"` event), `None` once the export settles (success or error) so a finished run never leaves a stale bar.

## progress_state

```rust
fn progress_state(pct: Option<u8>) -> tauri::window::ProgressBarState
```

Pure: the `ProgressBarState` for a given percent - split out from `set_export_progress` so the mapping itself is unit-testable without a real window.

### Inputs

- `pct: Option<u8>` - see `set_export_progress`.

### Returns

`Some(p)` -> `ProgressBarState { status: Some(Normal), progress: Some(p.min(100) as u64) }`. `None` -> `ProgressBarState { status: Some(None), progress: None }` (the `None` *status* variant is what actually hides the bar; `progress: None` alone would not).

### Behaviors

- `progress_state_some_is_normal_status_clamped_to_100` - `Some(250)` (exercising the clamp even though a real `u8` progress tick can't exceed 100 in practice) resolves to `Normal` status and `progress: Some(100)`.
- `progress_state_none_clears_the_bar` - `None` resolves to `status: Some(ProgressBarStatus::None)`, `progress: None`.

## decode_png

```rust
fn decode_png(bytes: &[u8]) -> Option<tauri::image::Image<'static>>
```

Decode PNG `bytes` into a Tauri `Image`, entirely in-process via the `png` crate (already a dependency, used elsewhere for the preview PNG encoder - `export::preview::png_encode`) rather than enabling Tauri's own `image-png` feature just for this one call site. *Why:* Tauri's `Image::from_bytes`/`from_path` are gated behind `#[cfg(feature = "image-png")]` (or `"image-ico"`), which isn't enabled on the `tauri` dependency in `Cargo.toml` - turning it on would pull in the `image` crate's whole format-decoder stack as new transitive weight, where a few lines against a dependency this crate already builds gets the identical result (an RGBA `Image`) without touching the "no new deps" rule at all.

### Inputs

- `bytes: &[u8]` - raw PNG file bytes (`ICON_NORMAL` or `ICON_REC`).

### Returns

`Some(Image)` on success; `None` on any decode failure OR if the decoded frame isn't exactly RGBA8 after the transform below (shouldn't happen for the bundled icons - both are RGBA8 already - but fails closed rather than misinterpreting bytes as a different layout).

### Implementation

1. `png::Decoder::new(bytes)`, `set_transformations(normalize_to_color8() | ALPHA)` - forces RGBA8 output regardless of the source PNG's actual color type (palette/grayscale/RGB would otherwise decode to their own native layout, since the decoder's default `Transformations::IDENTITY` does no normalization).
2. `read_info()` + `next_frame(&mut buf)` to get the pixel bytes and `OutputInfo` (width/height/color_type/bit_depth).
3. Verify `color_type == Rgba` and `bit_depth == Eight`; return `None` otherwise.
4. `Image::new_owned(buf, width, height)` - the `'static`, always-available constructor (unlike `from_bytes`, not feature-gated).

### Behaviors

- `both_bundled_icons_decode_to_nonzero_rgba` - decodes `ICON_NORMAL` and `ICON_REC`, asserting non-zero dimensions and that `rgba().len() == width * height * 4` for both - also a regression check that the two committed PNG assets are well-formed.
