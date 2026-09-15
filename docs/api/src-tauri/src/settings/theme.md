# src-tauri/src/settings/theme.rs

The one place a configured `ThemeMode` becomes a dark/light boolean.

## resolve_dark

```rust
pub fn resolve_dark(theme: ThemeMode, os_dark: bool) -> bool
```

`false` for `Light`, `true` for `Dark`, `os_dark` for `System`.

*Why the desktop preference is an argument and not a call.* This used to be `platform::resolve_dark(theme)`, which called `os_prefers_dark()` itself - a free function reaching straight into the registry from inside the render. Batch D deleted the free functions, so the OS read now happens at a composition root (`export::preview::with_warm_app` and `export::pipeline::run::run_export` resolve `Arc<Platform>` from app state and hand `SystemPort` down; `cursor_sprites` takes it as a command parameter) and this function stays pure - callable from a test with either answer, and one less thing an adapter has to exist for.

*Why it lives in `settings/` rather than beside the platform facade.* Only the OS read is platform code; mapping a user preference onto it is the same on every platform, and `platform/mod.rs` holds nothing but `Platform` and `current()` now.

### Inputs

- `theme: ThemeMode` - the user's configured preference, by value because it is `Copy`.
- `os_dark: bool` - what the desktop itself prefers, from `SystemPort::os_prefers_dark`. Read even when `theme` is not `System`, because the caller reads it before the match; it is one registry read per render build, not per frame.

### Implementation

1. `match theme`: three exhaustive arms, no default. *Why no default arm:* a new `ThemeMode` variant then fails to compile here instead of silently resolving to light.

### Behaviors

- `light_is_not_dark` - `Light` stays light even when the desktop is dark.
- `dark_is_dark` - `Dark` stays dark even when the desktop is light.
- `system_follows_the_desktop` - `System` returns the argument, both ways. This is the assertion the old `system_does_not_panic` could not make: with the OS read inlined, the test could only check that the call returned.

### Used by

- `src-tauri/src/export/render/mod.rs` (`FrameRenderer::new`) - drives dark-theme coloring in the render.
- `src-tauri/src/export/cursor/cursorpreview.rs` (`cursor_sprites`) - combined with `pack::theme_inverts` to decide whether to invert cursor sprites.
