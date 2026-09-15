# src-tauri/src/export/settings.rs

User-chosen export output settings: resolution (size), fps, quality (CRF), and container format. The single place `ExportSettings` maps onto pixel dimensions - `Layout::resolve` (`types.rs`) calls `rescale_to_resolution` (defined here) right after `apply_aspect`, so `Aspect` gives the RATIO and `Resolution` gives the SIZE, agreeing for every combination. Tests live in the sibling `settings_tests.rs` (`#[path]`-included), following the repo's file-size-limit test-split pattern.

## DEFAULT_CRF

```rust
pub const DEFAULT_CRF: u8 = 24
```

Today's export quality (`FfmpegFrameSink`'s old hardcoded "medium" CRF) - the default so an unconfigured export is unchanged.

## Resolution

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Resolution { P720, P1080, P1440, P2160, Source }
```

Output frame SIZE, independent of `Aspect` (the RATIO). Fixed presets name the SHORT edge - the smaller of width/height - in pixels: e.g. `P1080` on a landscape ratio (16:9, 4:3, 1:1) is height=1080 (matches the familiar "1080p"); on a portrait ratio (9:16) the short edge is the WIDTH, so `P1080` there is 1080x1920. `Source` (default, wire name `"source"`) is a no-op: dims stay whatever `apply_aspect` (or, for `Aspect::Source`, `adapt_to_source`) already resolved - today's exact behavior.

- `P720`/`P1080`/`P1440`/`P2160` (wire names `"p720"`/`"p1080"`/`"p1440"`/`"p2160"`) - short edge 720/1080/1440/2160 px.
- `Source` (default, `"source"`) - no size override.

### Used by

- `src-tauri/src/export/types.rs` - `Layout::resolve`'s new parameter, combined with `Aspect`.
- `src-tauri/src/export/render/mod.rs` - `FrameRenderer::new`'s new parameter (from `ExportSettings.resolution`, or `Resolution::Source` at every preview/test call site).

## Fps

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Fps { F30, F60, Source }
```

Output frame rate. `Source` reproduces today's exact fallback formula (the capture/display refresh rate, capped at 60) via `resolve_hz`; the default `F60` is a fixed 60 regardless of the display, which is what that fallback formula already produces on effectively every real machine (`primary_refresh_hz` rarely returns under 60) - so the default output is unchanged.

- `F30`/`F60` (wire `"f30"`/`"f60"`) - fixed output rate, ignoring the display.
- `Source` (default, `"source"`) - matches the capture/display refresh rate.

## Fps::resolve_hz

```rust
pub fn resolve_hz(self, capture_fps: u32) -> u64
```

Resolves to an actual encode rate (frames per second).

### Inputs

- `capture_fps: u32` - the SAME display-refresh-derived value `exporter::export` already computes for `FrameRenderer::new`'s capture-rate fallback. *Why reused rather than re-queried:* `Fps::Source` needs the identical value the old unconditional formula used, without a second display query.

### Returns

`u64` - `30` for `F30`; `60` for `F60` (always, regardless of `capture_fps`); for `Source`, `capture_fps as u64` unless it is `0`, in which case `60` (the exact `if fps == 0 {60} else {..}` guard the old formula used).

### Behaviors

- `fps_f60_is_always_60_regardless_of_capture_rate`, `fps_source_reproduces_the_old_fallback_formula`, `fps_f30_is_always_30`.

## Format

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Format { Mp4, WebM, Gif }
```

Export container/codec choice: `Mp4` (default, wire `"mp4"`, H.264), `WebM` (`"webm"`, VP9), `Gif` (`"gif"`, palettegen/paletteuse).

## Format::extension

```rust
pub fn extension(self) -> &'static str
```

Output file extension (`"mp4"`/`"webm"`/`"gif"`) - also names the `tmp_export.<ext>` intermediate.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` - names both the temp and (via `audio_mux::mux`) final output files.

## Format::supports_audio

```rust
pub fn supports_audio(self) -> bool
```

Whether this container can carry an audio track at all. `Gif` cannot (`false`), so `audio_mux::mux` always takes its "no audio" (rename-only) path for it, regardless of whether mic/system audio was recorded. `Mp4`/`WebM` are `true`.

## Format::audio_codec

```rust
pub fn audio_codec(self) -> &'static str
```

ffmpeg audio encoder name used by the final mux step: `"aac"` for `Mp4`, `"libopus"` for `WebM`, `""` for `Gif` (never read, since `supports_audio` is `false`).

### Behaviors

- `format_extensions_and_audio_support` - covers all three formats' extension + `supports_audio`.

## ExportSettings

```rust
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct ExportSettings { pub resolution: Resolution, pub fps: Fps, pub quality_crf: u8, pub format: Format }
```

User-chosen export settings, collected by `ExportDialog` (frontend) and threaded through `export_project` -> `run_export` -> `exporter::export` into `Layout` (dims), the encode loop (fps), and `FfmpegFrameSink`/`audio_mux::mux` (quality + container). `#[serde(default)]` means a partially-specified settings object (or, defensively, an empty one) still fills in every missing field from `Default`.

- `resolution: Resolution` - output SIZE.
- `fps: Fps` - output frame rate.
- `quality_crf: u8` - CRF slider, 18..28 (see `Fps`/encoder docs for exactly which encoders honor it).
- `format: Format` - container/codec.

### Behaviors

- `default_export_settings_matches_todays_export` - `Default` is exactly `{Source, F60, DEFAULT_CRF, Mp4}`.
- `serde_defaults_fill_in_missing_fields` - `serde_json::from_str("{}")` deserializes to `Default`.

### Used by

- `src-tauri/src/commands.rs` - `export_project`'s new parameter.
- `src-tauri/src/export/pipeline/run.rs` - `run_export`'s new parameter, threaded to `exporter::export`.
- `src/shared/ipc.ts` - mirrored as the TS `ExportSettings` interface.

## Layout::rescale_to_resolution

```rust
pub(crate) fn rescale_to_resolution(&mut self, resolution: Resolution)
```

Rescales the already aspect-resolved `out_w`/`out_h` to `resolution`'s SHORT edge, preserving the exact ratio `apply_aspect` produced (see `Resolution`'s own doc for the short-edge convention). Called from `Layout::resolve` (`types.rs`) right after `apply_aspect`, so export/preview callers never invoke this directly. A no-op for `Resolution::Source`.

### Behaviors

- `resolution_source_is_a_noop_on_layout` - `Source` leaves `out_w`/`out_h` untouched.
- `resolution_presets_use_the_short_edge_convention` - a landscape (16:9) ratio's short edge is the height; a portrait (9:16) ratio's short edge is the width.
- `resolution_p1080_matches_every_fixed_aspect_preset_exactly` - cross-check: the 4 fixed `Aspect` presets are already anchored at a 1080 short edge (long edge 1920), so combining any of them with `Resolution::P1080` reproduces that same preset's dims exactly.

### Used by

- `src-tauri/src/export/types.rs` - `Layout::resolve` calls this right after `apply_aspect`.
