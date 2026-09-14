# src-tauri/src/export/pipeline/exporter_bench.rs

The manual export benchmark, split out of `exporter.rs` when that file reached its 200-line budget. Test-only (`#[cfg(test)] #[path] mod bench;`), so it ships in no binary.

## preview_frame_bench

```rust
#[test]
#[ignore]
fn preview_frame_bench()
```

One composited frame of a real recording to `%TEMP%/tcursor-preview-frame.png`, through the editor's own `render_preview` (the GPU compositor; no video encoder is involved). `TCURSOR_REC` names the folder, `TCURSOR_MS` the instant (default 1500):

```
TCURSOR_REC=C:\path\to\a\recording TCURSOR_MS=2500 cargo test preview_frame_bench -- --ignored --nocapture
```

Made for 2026-09-14's odd-sized capture, whose export slid and sheared: a look at one frame says whether a decode is sound without occupying the machine's hardware encoder for a full export (which, run while the owner was recording, had already cost one take its video).

## export_bench

```rust
#[test]
#[ignore]
fn export_bench()
```

Runs a FULL export of a real recording folder and prints how long it took. `#[ignore]`d because it needs a recording that only exists on a developer's machine and takes minutes:

```
TCURSOR_REC=C:\path\to\a\recording cargo test export_bench -- --ignored --nocapture
```

Reads the folder from `TCURSOR_REC` (panics with that instruction if unset), builds a `ProjectPaths` from it, and calls `exporter::export` with `ExportSettings::default()` and a no-op progress callback. The per-stage breakdown it is usually paired with is the one `export` writes to `%TEMP%/tcursor-export-timing.txt`.

### Used by

Nothing in the build - it is an opt-in measuring tool, not a regression test.
