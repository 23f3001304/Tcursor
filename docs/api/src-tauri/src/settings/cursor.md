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
- `src-tauri/src/export/cursor/pack/cursorset.rs` (`prep`) - returns `None` early for any style except `Enhanced`, skipping sprite preparation

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
    #[serde(default = "default_tilt")]
    pub tilt: f32,
    pub click_bounce: bool,
    pub bounce_intensity: f32,
    pub pack: String,
}
```

Per-cursor appearance and animation settings. Applies only when `style == Enhanced`. No longer `Copy` (the `pack` field is a `String`) - callers that need an owned copy alongside a live borrow now `.clone()` explicitly.

Fields:

- `style: CursorStyle` - rendering mode. Default `CursorStyle::System`.
- `size: f32` - scale factor applied to the base cursor sprite size (1.0 = default). Default `1.0`. *Why:* lets users with high-DPI recordings scale the cursor up so it remains readable on smaller playback screens.
- `smoothness: f32` - 0..1 glide amount for the moves between the recording's rests (0 = the recording's own timing and jitter; 1 = one clean eased stroke per move). Default `0.6`. Read by `Cursor` through `smoothness_at` (below) and consumed by `export/cursor/path.rs`, which re-times and de-jitters each move with both ends pinned - so at any value the cursor is exactly where the hand rested and clicked (2026-09-14; it used to be a lagging low-pass alpha). This is the CURSOR glide, distinct from `ZoomSettings::camera_smoothing_ms` (see `model.md`), which smooths the auto-zoom CAMERA path instead.
- `path_idealize: f32` - 0..1 amount by which the route of each move between rests is straightened toward its chord (0 = off, the raw route). Default `0.0` (off - byte-identical to recordings made before this field existed). *Why off by default:* straightening is a stylistic choice, not a correctness fix, so recordings shouldn't change appearance until a user opts in. Like `smoothness`, it never moves a rest or a click.
- `motion_blur: f32` - trail strength for the motion blur effect (0 = off, 1 = maximum). Default `0.35`. *Why 0.35:* noticeable but not overwhelming on fast pans; zero would make the sprite look teleporting.
- `tilt: f32` - 0..1 **motion lean**: how far a fast cursor tips into its own travel, and overshoots once coming back upright when it stops (`export/cursor/draw/tilt.rs`). Scales the 6-degree cap (`tilt::max_deg`); 0 switches the filter off entirely, so an upright cursor costs nothing at all. Default `0.35` (`default_tilt`). *Why present by default, unlike `path_idealize`:* this is the effect the owner asked for and it is what other editors do; *why only a third of the cap:* they asked for "very subtle", and a lean you notice as a lean is already too much. Read by `FrameRenderer::new`/`reload_edit` through `tilt_at`, and mirrored live by `src/editor/stage/cursor/cursorTilt.ts`.
- `click_bounce: bool` - whether the cursor sprite plays a bounce-dip animation on mouse down. Default `true`. *Why on by default:* the bounce makes click detection trivially legible without any visual effect ring.
- `bounce_intensity: f32` - depth of the bounce dip on a 0..1 scale (0.5 gives approx 0.18 scale-dip, 1.0 gives approx 0.36 dip). Default `0.5`. *Why not 1.0:* a full dip at 1.0 looks cartoonish; 0.5 gives a subtle-but-readable response.
- `pack: String` - which cursor sprite pack draws the Enhanced cursor. Default `"default"` (the built-in embedded set - byte-identical to every recording made before this field existed). Any other value is an imported pack id (`export/cursor/pack.rs`'s `CursorPackInfo.id`); a pack missing a given kind's PNG falls back to the built-in sprite for just that kind. *Why a plain `String` id rather than an enum:* imported packs are discovered at runtime from the filesystem, so the set of valid values isn't known at compile time.

### Used by

- `src-tauri/src/settings/model.rs` (`Settings.cursor`) - persisted in `config.json`
- `src-tauri/src/export/cursor/pack/cursorset.rs` (`prep`) - reads `style` to gate, and `pack` (via `export::cursor::pack::sprite_sources`) to resolve which sprite bytes to decode, plus `motion_blur`/`click_bounce`/`bounce_intensity` to configure animation
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`) - resolves `pack` the same way so the editor preview matches the export
- `src-tauri/src/session/record/recorder.rs` - reads `style.captures_os_cursor()` and checks `style == Enhanced` to initialize the cursor type tracker
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`, `FrameRenderer::reload_edit`) - reads `smoothness_at()` to set the owned `Cursor`'s glide and `idealize_at()` (via `Cursor::set_idealize`) to configure path straightening; `reload_edit` live-applies BOTH on an `edit.json` change (`Cursor::set_smoothness`/`set_idealize`) without a full renderer rebuild
- `src/editor/panels/cursor/CursorPanel.tsx` - lists/selects/imports packs, writing the chosen id into `pack`

## CursorSettings::plain_os

```rust
pub fn plain_os(&self, os_cursor_in_video: bool) -> bool
```

Whether the synthetic cursor must be drawn in **plain-OS mode**: the doc asks for `System`, but the recorded video has no baked OS cursor to show. True only for that combination - `Enhanced` and `Hidden` are never plain-OS, and `System` on a video that *does* have the cursor baked in stays "draw nothing" as before.

*Why `os_cursor_in_video` is a parameter:* it is RECORD-time truth (see `settings::store::os_cursor_in_video`). `self.style` is the editable doc's value - the very thing the user changed - so it can never answer this question on its own.

*What plain-OS mode does:* the renderer draws the Arrow sprite along the raw recorded path, with no smoothing, no path idealization, no click bounce and no motion trail - "the real cursor", re-created from the data the recording does have. `cursorset::draw` applies the sprite/bounce/trail half; the two helpers below apply the path half.

## CursorSettings::smoothness_at

```rust
pub fn smoothness_at(&self, os_cursor_in_video: bool) -> f32
```

`smoothness` (clamped 0..1), or `0.0` in plain-OS mode. *Why `0.0`:* with no smoothness and no idealization `Cursor::at` returns the interpolated sample verbatim - the raw recorded path, with the glide switched off rather than merely reduced.

## default_tilt

```rust
fn default_tilt() -> f32
```

`0.35` - `tilt`'s default, as both `CursorSettings::default()` and the **field-level** `#[serde(default)]`. *Why a field-level default when the struct already carries `#[serde(default)]`:* the struct-level one only fills in a `cursor` block that is missing **entirely**. Every project saved before this field existed has a `cursor` block without it, and serde would otherwise give that field `f32::default()` - `0.0`, i.e. the feature silently off for every existing recording. The two defaults are pinned equal by `the_motion_tilt_round_trips_and_defaults_present_but_subtle`.

## CursorSettings::tilt_at

```rust
pub fn tilt_at(&self, os_cursor_in_video: bool) -> f32
```

`tilt`, or `0.0` in plain-OS mode. *Why:* the lean is fake polish, and a re-created system cursor promises none of it - the same rule `smoothness_at` and `idealize_at` apply to the path, and `cursorset::draw` re-asserts at draw time. Applied by `FrameRenderer::new` and, so a live style switch takes effect without a rebuild, by `reload_edit` (both through `Cursor::set_tilt`).

## CursorSettings::idealize_at

```rust
pub fn idealize_at(&self, os_cursor_in_video: bool) -> f32
```

`path_idealize`, or `0.0` in plain-OS mode (no straightening of the moves between rests). Applied by `FrameRenderer::new` and, so a live style switch takes effect without a rebuild, by `reload_edit`.

### Used by

- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`) - `smoothness_at` is passed to `Cursor::new` as the initial glide, `idealize_at` to `Cursor::set_idealize`
- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::reload_edit`) - passed to `Cursor::set_smoothness`/`set_idealize` to live-apply a slider change without rebuilding the renderer

## CursorBack

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CursorBack {
    #[default]
    None,
    Glass,
}
```

The glass shape drawn BEHIND the cursor, whatever pack it comes from.

- `None` - default, and the original look: nothing behind the sprite.
- `Glass` - a refracting disc that MORPHS by cursor kind: a horizontal pill over text, stretching into a selection bar while the left button is held there, easing over `fx_lens::MORPH_MS`. Rendered by `fx_lens.wgsl` (GPU) or `fx_lensdraw.rs` (CPU), placed by `fx_lensbuild::lenses_at`.

*Why a setting rather than a pack property:* it is independent of the pack's own `material`. A plain pack can have a glass back, and a glass pack can go without - the pack decides the cursor's shape, this decides what sits behind it. The editor's Cursor panel puts the two controls next to each other for exactly that reason.

*Why it defaults off:* it changes the look of every existing project's cursor, and the honest default for a setting like that is the look the project already had.

Serialises as lowercase (`"none"` / `"glass"`), mirrored by TS `CursorBackStyle` in `src/hud/settings/settings.ts`. `CursorSettings` carries `#[serde(default)]`, so every settings file written before this existed loads with the back off rather than failing.

### Used by

- `src-tauri/src/export/fx/lens/build.rs` - `lenses_at` / `wants_lens`, the only readers.
- `src/editor/panels/cursor/CursorPanel.tsx` and `src/hud/settings/SettingsCursor.tsx` - the two pickers.
