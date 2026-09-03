# src-tauri/src/edit/ops/metrics.rs

Read-only summary statistics over an `EditDoc`. Moved out of `api.rs` when that file reached the size budget: it is a separate responsibility from the mutating ops there, and the only thing in `edit::ops` that never touches the doc.

## Metrics

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Metrics {
    pub duration_ms: u32,
    pub kept_ms: u32,
    pub zoom_count: usize,
    pub cut_count: usize,
}
```

Read-only doc summary. Derived on demand from `EditDoc`; never stored.

- `duration_ms` - *raw clip length (`trim.out_ms`); sets the timeline ruler's right edge.*
- `kept_ms` - *`(trim.out - trim.in) - sum(cut durations within trim)`; drives the export file-size estimate shown in the sidebar.*
- `zoom_count` - *`doc.zooms.len()`; the editor badge indicating how many zoom events are active.*
- `cut_count` - *`doc.cuts.len()`; pairs with `kept_ms` so the user can confirm cuts were registered.*

## metrics

```rust
pub fn metrics(doc: &EditDoc) -> Metrics
```

Derives a `Metrics` snapshot from the current state of `doc` without mutating it.

### Inputs

- `doc: &EditDoc` - the document to measure. *Why immutable ref:* `metrics` is a pure read; it must never be callable in a context that expects mutation.

### Returns

`Metrics` with all four fields computed in a single pass.

### Implementation

1. `duration_ms = doc.trim.out_ms`.
2. `trim_span = trim.out_ms - trim.in_ms` (saturating).
3. For each cut, clamp it to `[trim_in, trim_out]`, compute clamped length, accumulate. *Why clamp:* cuts outside the trim window do not reduce the exported clip.
4. `kept_ms = trim_span - cut_sum` (saturating).
5. `zoom_count = doc.zooms.len()`, `cut_count = doc.cuts.len()`.

### Behaviors

- `metrics_kept_ms_subtracts_cuts` - a 10 s clip (`SetTrim {0, 10000}`) with a 2 s cut (`AddCut {1000, 3000}`) yields `kept_ms=8000`, `cut_count=1`.
- `a_cut_outside_the_trim_window_does_not_reduce_kept_ms` - a cut wholly past the trim leaves `kept_ms` alone; a cut straddling `trim.in_ms` only subtracts its overlapping part.
