# src-tauri/src/main.rs

The binary's six lines: a `windows_subsystem = "windows"` attribute for release builds (no console window behind the app) and a `main` that calls `cursor_zoom_lib::run()`. Everything else lives in the library crate (`lib.md`), which is what lets `cargo test` and the manual probes link the whole app without a second entry point.

## main

```rust
fn main()
```

Calls `cursor_zoom_lib::run()`. The `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` above it is load-bearing: without it a release build opens a console window next to the HUD. The comment in the source says DO NOT REMOVE for that reason.

### Used by

- Cargo, as the `cursor-zoom` binary target.
