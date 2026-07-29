# src-tauri/src/ai/backend/narrate.rs

Turns each `EditOp` from the AI director's plan into a short, human "what I did + why" sentence for the agentic reveal log - the frontend applies one plan step at a time and shows this line alongside it, so auto-edit reads like a live agent narrating its own decisions rather than a plan dumped all at once. Zooms are correlated back to the click that triggered them (reusing `timeline::region`, the same classifier the transcript itself uses) so a zoom's narration names the same region the LLM already saw.

## label_for

```rust
pub fn label_for(op: &EditOp, log: &EventLog, dur_ms: u32) -> String
```

Builds one narration line for one plan step.

### Inputs

- `op: &EditOp` - the operation being narrated. *Why match the full enum rather than a narrower type:* `label_for` is called once per step of an `ai_plan` result, in application order, so it must handle every variant the AI plan can produce (`ClearZooms`, `AddZoomFull`, `AddZoom`, `SetTrim`) plus a catch-all for everything else.
- `log: &EventLog` - the recording's mouse event log; supplies both the click list (to find the one nearest a zoom's `at_ms`) and `log.screen.{w,h}` (for `region()`). *Why the whole log rather than just a click list:* `zoom_label` needs both pieces, and splitting them into separate parameters would just re-derive what `EventLog` already holds together.
- `dur_ms: u32` - the clip's true duration, needed by `trim_label` to tell a real tail trim from a no-op `out_ms` (`0` or `>= dur_ms` both mean "no tail trim").

### Returns

A one-line `String`, already formatted for direct display in the reveal log - the caller does no further templating.

### Implementation

Matches on `op`:

1. `ClearZooms` -> the fixed string `"Rethinking your zooms…"`. *Why fixed rather than computed:* this step never varies - it always means "the mechanical auto-zooms are about to be replaced." Note that `ai::commands::ai_plan` currently never routes a `ClearZooms` op through `label_for` (see Used by) - this arm exists for completeness and for the unit tests below to exercise directly.
2. `AddZoomFull { at_ms, scale, .. }` -> `zoom_label(*at_ms, *scale, log)`.
3. `AddZoom { at_ms, .. }` -> `zoom_label(*at_ms, 2.0, log)`. *Why hardcode `2.0`:* `AddZoom` has no `scale` field of its own (the api layer always applies its 2.0 default for that variant), so the narration mirrors that default instead of reading a value that doesn't exist.
4. `SetTrim { in_ms, out_ms }` -> `trim_label(*in_ms, *out_ms, dur_ms)`.
5. Every other variant -> the generic fallback `"Applied an edit"`. *Why a catch-all:* `label_for` only needs to narrate what the AI director's own plan can produce (`ai::backend::plan::ops_from_json` only ever emits `AddZoomFull` and `SetTrim`); all other `EditOp` variants (camera moves, effects, layout segments, manual zoom edits, ...) come from the hand-editor, which doesn't run through the agentic reveal.

Two private helpers do the actual formatting:

- `zoom_label(at_ms: u32, scale: f32, log: &EventLog) -> String` - finds the `EventKind::Down` event nearest `at_ms` via `min_by_key` on absolute millisecond distance, then keeps it only if that distance is `<= 600`. If a near click exists: `"Zoomed into the <region> — you clicked there · <mm:ss>"`, where `<region>` is `timeline::region(click.x, click.y, log.screen.w, log.screen.h)` and `<mm:ss>` is `mmss(at_ms)`. Otherwise (no click within 600 ms - e.g. a zoom the AI placed on a typing burst instead of a click): `"Zoomed in at <mm:ss> · <scale>×"`, `scale` formatted to one decimal place. *Why a 600 ms window:* wide enough to survive the AI's own "start up to 0.3s before the click" placement rule (`prompt::system_prompt`) plus slack for clock rounding, narrow enough that an unrelated click a second or two away isn't misattributed as the cause.
- `trim_label(in_ms: u32, out_ms: u32, dur_ms: u32) -> String` - `head = in_ms`; `tail = 0` when `out_ms == 0 || out_ms >= dur_ms` (both mean "no tail trim"), else `dur_ms - out_ms`. Both are rendered to one-decimal seconds (e.g. `2000` -> `"2.0s"`). Branches on `(head > 0, tail > 0)`: both -> `"Trimmed <head> off the start and <tail> off the end"`; head only -> `"Trimmed <head> of dead air off the start"`; tail only -> `"Trimmed <tail> of dead air off the end"`; neither -> `"Kept the full clip"`. *Why "dead air" only appears in the single-sided messages:* trimming just one end reads like removing filler; trimming both ends already reads as intentional, so repeating "dead air" on both sides would be redundant.
- `mmss(ms: u32) -> String` - `"<minutes>:<seconds>"`, seconds zero-padded to two digits (e.g. `68_000` -> `"1:08"`).

### Behaviors

- `zoom_near_a_click_names_the_region_and_time` - a zoom 100 ms after a top-left click names both `"top-left"` and `"clicked"`, and formats the time as `"0:03"`.
- `zoom_far_from_clicks_falls_back_to_time_and_scale` - a zoom 68 s after the only click (well outside the 600 ms window) falls back to the `mm:ss · scale×` form, containing both `"1:08"` and `"2.4"`.
- `trim_head_and_tail_both_named` - a trim with both a head and a tail gap mentions `"start"` and `"end"`.
- `trim_out_zero_means_head_only` - `out_ms: 0` mentions `"start"` but not `"end"`, confirming the "`0` means no tail trim" convention.

### Used by

- `src-tauri/src/ai/commands.rs` - `ai_plan` calls `label_for` for every op in the LLM-produced plan to build that step's `AiStep.label`. The leading `ClearZooms` step that `ai_plan` itself prepends (only when the doc already has zooms) is labeled inline with the same fixed string rather than through `label_for`, since `plan::ops_from_json` never produces a `ClearZooms` op for `label_for` to see in practice.
