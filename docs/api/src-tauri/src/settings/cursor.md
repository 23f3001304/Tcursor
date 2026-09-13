# src-tauri/src/settings/cursor.rs

Cursor rendering mode and animation settings, split out of `settings/model.rs` (which still holds every other settings struct/enum, including the top-level `Settings` struct that embeds `CursorSettings` as `Settings.cursor`) purely to keep that file under the repo's 200-line-per-file limit. Carries `#[serde(default)]` like every other settings struct, so old config files gain new fields silently.

## CursorStyle

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle { System, Enhanced, Hidden }
```

Controls how the cursor appears in the output video.

- `System` - the OS cursor is baked directly into the Windows Graphics Capture frame. Default. *Why default:* zero-latency, zero-overhead; the OS composites the cursor for free and the result is always pixel-perfect.
- `Enhanced` - the OS cursor is excluded from WGC capture; TCursor draws its own sprite with motion blur, bounce animations, and per-type icons. *Why excluded rather than replaced:* drawing on top of the captured cursor would cause a double cursor; exclusion ensures only the enhanced sprite appears.
- `Hidden` - the OS cursor is excluded and nothing is drawn in its place. *Why:* suitable for recordings where the presenter moves the mouse for internal navigation but does not want the cursor on screen.

Serialises as lowercase.

### Used by

- `src-tauri/src/session/record/recorder.rs` - calls `captures_os_cursor()` to set the WGC cursor inclusion flag; checks `== Enhanced` to start the `CursorTypeTracker`
- `src-tauri/src/settings/store.rs` (`os_cursor_in_video`) - calls `captures_os_cursor()` on the record-time snapshot
- `src-tauri/src/export/cursor/cursorset.rs` (`prep`) - returns `None` early for any style except `Enhanced`, skipping sprite preparation

## CursorStyle::captures_os_cursor

```rust
pub fn captures_os_cursor(self) -> bool
```

Returns `true` only for `CursorStyle::System`.

**Legacy detection only.** This is no longer a capture decision: recording excludes the OS cursor for EVERY style (`recorder.rs` passes `with_cursor: false` unconditionally) and captures the real cursor as its own layer instead, which is what makes the style switchable in the editor. What the method now identifies is a recording made BEFORE that change - back then, and only back then, `System` meant the cursor was baked into the pixels. It is therefore always paired with a "does this project have a cursor layer" check; see `settings::store::os_cursor_in_video`.

### Inputs

- `self: CursorStyle` - the RECORD-TIME cursor style, read from the frozen `settings.json` snapshot. Calling it on the editable doc's style would be meaningless - the whole point is that the doc's style can now differ from the one the take was recorded with.

### Returns

`true` for `System`; `false` for `Enhanced` and `Hidden`.

### Behaviors

- `partial_json_fills_defaults` in `model_tests.rs` - asserts `System.captures_os_cursor() == true`, `Enhanced.captures_os_cursor() == false`, `Hidden.captures_os_cursor() == false`.

## CursorSettings

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CursorSettings {
    pub style: CursorStyle,
    pub size: f32,
    pub smoothness: f32,
    pub path_idealize: f32,
    pub motion_blur: f32,
    pub click_bounce: bool,
    pub bounce_intensity: f32,
    pub pack: String,
}
```

Per-cursor appearance and animation settings. Applies only when `style == Enhanced`. No longer `Copy` (the `pack` field is a `String`) - callers that need an owned copy alongside a live borrow now `.clone()` explicitly.

Fields:

- `style: CursorStyle` - rendering mode. Default `CursorStyle::System`.
- `size: f32` - scale factor applied to the base cursor sprite size (1.0 = default). Default `1.0`. *Why:* lets users with high-DPI recordings scale the cursor up so it remains readable on smaller playback screens.
- `smoothness: f32` - 0..1 cursor-follow glide amount (0 = snappy/raw, tracks the real cursor closely; 1 = glassy, heavily damped). Default `0.6`. Not used directly - converted to the actual low-pass alpha via `follow_alpha()` (below), so the UI's 0..1 range maps onto a usefully-shaped alpha curve rather than a linear one. *Why default 0.6:* resolves to alpha `~0.36`, matching the old hardcoded smoothing constant, so existing recordings/configs render identically until a user touches the slider. This is the CURSOR low-pass, distinct from `ZoomSettings::camera_smoothing_ms` (see `model.md`), which smooths the auto-zoom CAMERA path instead.
- `path_idealize: f32` - 0..1 amount by which wandering cursor paths are straightened into clean strokes between clicks (0 = off, the raw path). Default `0.0` (off - byte-identical to recordings made before this field existed). *Why off by default:* straightening is a stylistic choice, not a correctness fix, so recordings shouldn't change appearance until a user opts in.
- `motion_blur: f32` - trail strength for the motion blur effect (0 = off, 1 = maximum). Default `0.35`. *Why 0.35:* noticeable but not overwhelming on fast pans; zero would make the sprite look teleporting.
- `click_bounce: bool` - whether the cursor sprite plays a bounce-dip animation on mouse down. Default `true`. *Why on by default:* the bounce makes click detection trivially legible without any visual effect ring.
- `bounce_intensity: f32` - depth of the bounce dip on a 0..1 scale (0.5 gives approx 0.18 scale-dip, 1.0 gives approx 0.36 dip). Default `0.5`. *Why not 1.0:* a full dip at 1.0 looks cartoonish; 0.5 gives a subtle-but-readable response.
- `pack: String` - which cursor sprite pack draws the Enhanced cursor. Default `"default"` (the built-in embedded set - byte-identical to every recording made before this field existed). Any other value is an imported pack id (`export/cursor/pack.rs`'s `CursorPackInfo.id`); a pack missing a given kind's PNG falls back to the built-in sprite for just that kind. *Why a plain `String` id rather than an enum:* imported packs are discovered at runtime from the filesystem, so the set of valid values isn't known at compile time.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.cursor`) - persisted in `config.json`
- `src-tauri/src/export/cursor/cursorset.rs` (`prep`) - reads `style` to gate, and `pack` (via `export::cursor::pack::sprite_sources`) to resolve which sprite bytes to decode, plus `motion_blur`/`click_bounce`/`bounce_intensity` to configure animation
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`) - resolves `pack` the same way so the editor preview matches the export
- `src-tauri/src/session/record/recorder.rs` - reads `style.captures_os_cursor()` and checks `style == Enhanced` to initialize the cursor type tracker
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`, `FrameRenderer::reload_edit`) - reads `follow_alpha()` to set the owned `Cursor`'s low-pass alpha and `path_idealize` (via `Cursor::set_idealize`) to configure path straightening; `reload_edit` live-applies BOTH on an `edit.json` change (`Cursor::set_a`/`set_idealize`) without a full renderer rebuild
- `src/editor/panels/CursorPanel.tsx` - lists/selects/imports packs, writing the chosen id into `pack`

## CursorSettings::follow_alpha

```rust
pub fn follow_alpha(&self) -> f32
```

Converts the user-facing `smoothness` (0..1) into the actual low-pass alpha the renderer's `Cursor` uses to damp cursor motion.

### Inputs

- `self` - the `CursorSettings` snapshot. *Why a method:* the mapping is a fixed formula independent of any other settings, so it belongs next to the field it derives from rather than being duplicated at each call site.

### Returns

`0.75 - 0.65 * self.smoothness.clamp(0.0, 1.0)` - `smoothness = 0` gives alpha `0.75` (snappy, follows the raw cursor closely); `smoothness = 1` gives alpha `0.10` (glassy glide). Default `smoothness = 0.6` gives alpha `~0.36`, matching the old hardcoded smoothing constant (`0.35`), so a fresh config renders like every recording made before this field existed.

## CursorSettings::plain_os

```rust
pub fn plain_os(&self, os_cursor_in_video: bool) -> bool
```

Whether the synthetic cursor must be drawn in **plain-OS mode**: the doc asks for `System`, but the recorded video has no baked OS cursor to show. True only for that combination - `Enhanced` and `Hidden` are never plain-OS, and `System` on a video that *does* have the cursor baked in stays "draw nothing" as before.

*Why `os_cursor_in_video` is a parameter:* it is RECORD-time truth (see `settings::store::os_cursor_in_video`). `self.style` is the editable doc's value - the very thing the user changed - so it can never answer this question on its own.

*What plain-OS mode does:* the renderer draws the Arrow sprite along the raw recorded path, with no smoothing, no path idealization, no click bounce and no motion trail - "the real cursor", re-created from the data the recording does have. `cursorset::draw` applies the sprite/bounce/trail half; the two helpers below apply the path half.

## CursorSettings::follow_alpha_at

```rust
pub fn follow_alpha_at(&self, os_cursor_in_video: bool) -> f32
```

`follow_alpha()`, or `1.0` in plain-OS mode. *Why `1.0`:* `Cursor::at`'s low pass is `s += (raw - s) * a`, so `a = 1.0` returns the interpolated sample verbatim - the raw recorded path, with the glide switched off rather than merely reduced.

## CursorSettings::idealize_at

```rust
pub fn idealize_at(&self, os_cursor_in_video: bool) -> f32
```

`path_idealize`, or `0.0` in plain-OS mode (no straightening toward the click anchors). Applied by `FrameRenderer::new` and, so a live style switch takes effect without a rebuild, by `reload_edit`.

### Used by

- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`) - passed to `Cursor::new` as the initial low-pass alpha
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::reload_edit`) - passed to `Cursor::set_a` to live-apply a `smoothness` slider change without rebuilding the renderer
