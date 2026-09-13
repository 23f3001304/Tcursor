# src-tauri/src/export/pipeline/exporter_bench.rs

The manual export benchmark, split out of `exporter.rs` when that file reached its 200-line budget. Test-only (`#[cfg(test)] #[path] mod bench;`), so it ships in no binary.

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
