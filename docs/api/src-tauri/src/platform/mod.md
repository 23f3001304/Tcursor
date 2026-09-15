# src-tauri/src/platform/mod.rs

MODULE OVERVIEW: The composition root. `platform` holds one bundle of adapters satisfying the `crate::ports` traits, chosen for the target this binary was built for, and `current()` holds the ONE `#[cfg]` that picks a set. Since Batch D the file holds nothing else: the bundle is built in `lib.rs`'s `setup`, managed as `Arc<Platform>`, and every OS-facing call in the crate reaches an adapter through it.

The five transitional free functions that lived here through Batch C - `primary_refresh_hz`, `os_prefers_dark`, `resolve_dark`, `exclude_from_capture`, `loopback_device` - are gone. Four of them were `#[cfg]` wrappers whose callers now take `State<'_, Arc<Platform>>` or are handed a `&dyn SystemPort` by whoever does; the fifth, `resolve_dark`, made no OS call at all and moved to `settings/theme.rs` with the desktop preference as an argument.

## windows

The Windows adapters. `#[cfg(windows)]`, and - with `current()` - the last place in the crate that is conditional on a platform.

## mock

`#[cfg(test)]`. A bundle whose ports answer without an OS: empty input tracks, a fixed `CaptureGeometry`, and a sink that records the calls made to it. It is what makes the recorder's start/pause/resume/stop cycle testable headlessly. See `platform/mock/mod.md`.

## Platform

```rust
pub struct Platform {
    pub capture: Box<dyn CapturePort>,
    pub input: Box<dyn InputPort>,
    pub system: Box<dyn SystemPort>,
    pub audio: Box<dyn SystemAudioPort>,
}
```

Every OS-facing capability the app has, in one value.

*Why one bundle and not four managed states.* It is process state, created once and shared, not per recording. Studio is a mode inside this same binary with its own launcher window and one `config.json`, so a second window must reach the same adapters as the first; building a bundle per take would also mean a take could disagree with the app about which platform it was on.

`Box` rather than `Arc` on the fields because the bundle itself is what gets shared (as `Arc<Platform>` in app state); the ports inside it have exactly one owner.

*What takes the whole bundle and what takes one port.* A Tauri command takes `tauri::State<'_, Arc<Platform>>` and reaches the port it needs. Code below the command layer takes the single port instead - `export::render::FrameRenderer::new`, `export::preview::build_renderer` and `export::pipeline::exporter::export` all take `&dyn SystemPort`, not a `Platform` - so the export declares the one OS fact it depends on rather than inheriting the right to read any of them.

### Used by

- `src-tauri/src/lib.rs` (`run` setup) - built here as `Arc<Platform>`, `manage`d, and used for the startup capture exclusion before anything else in `setup` runs.
- `src-tauri/src/session/record/recorder.rs` (`start_take`) - the refresh rate, all three input streams, the loopback device and the capture start for one take.
- `src-tauri/src/session/record/switch_display.rs`, `src-tauri/src/commands.rs` (`list_displays`, `set_capturable`), `src-tauri/src/export/preview/session.rs` (`with_warm_app`), `src-tauri/src/export/pipeline/run.rs` (`run_export`), `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`).

## current

```rust
pub fn current() -> Platform
```

The adapter bundle for the platform this binary runs on. Built once, in `lib.rs`'s `setup`, and shared as app state from there. The `#[ignore]`d benches in `export/` construct their own, because they run outside the app.

On a target with no adapter yet - which Phase 1 leaves as everything but Windows - this is `unimplemented!`. *Why a panic rather than a stub bundle:* a stub would let a build that cannot record link and start, and the cross-compile `cargo check` that keeps the ports honest is a compile check, not a run. Reaching here is a build configuration mistake, not a runtime condition to recover from.
