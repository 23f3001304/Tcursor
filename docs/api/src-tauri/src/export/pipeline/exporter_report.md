# src-tauri/src/export/pipeline/exporter_report.rs

What the export SAYS about itself while it runs: the progress callback's rate-limiting, the periodic speed line, and the per-stage timing breakdown written at the end. Split out of `exporter.rs` for its line budget. Pure reporting - nothing here affects a single pixel of the output.

## tick_progress

```rust
pub(super) fn tick_progress(done: u64, total_out: u64, last_pct: u8, start: Instant,
                            on_progress: &impl Fn(u8)) -> u8
```

Report one completed frame. Returns the percentage now showing, which the caller keeps as its `last_pct`.

*Why the caller holds the state:* the rate-limiting IS the state - `on_progress` fires only when the whole number changes, so a 30-minute export calls into the frontend 100 times instead of 100,000 (each call crosses the Tauri IPC boundary and re-renders the export dialog). Returning the new value keeps this function pure enough to read at a glance.

`done` counts completed frames (1-based), so it - unlike the raw index `k - k_in` - reaches exactly `total_out` on the last frame actually sent, i.e. a real 100%.

The stderr speed line is every 5%, not every 1%: enough to watch a long export's pace without turning stderr into the export's own bottleneck.

## log_timing

```rust
pub(super) fn log_timing(total_out: u64, secs: f64, t_dec: u128, t_comp: u128, t_send: u128)
```

The finishing line plus `%TEMP%/tcursor-export-timing.txt`.

The three stage totals are microseconds spent BLOCKED in each stage (decode channel, composite, encoder channel send). Because the three stages overlap through bounded channels, the wall-clock total trends toward `max(decode, composite)` rather than their sum - so the useful signal is a lopsided pair, not the absolute numbers.

### Used by

- `src-tauri/src/export/pipeline/exporter.rs` (`export`) - the only caller of both.
