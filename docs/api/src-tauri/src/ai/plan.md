# src-tauri/src/ai/plan.rs

Turns a local LLM's raw chat reply into safe, clamped `EditOp` values. This is the trust boundary between untrusted model output and the edit document - all numeric fields are validated against the clip duration to prevent out-of-bounds access or integer overflow, and the private `extract_json` helper tolerates markdown fences the model may add despite instructions to the contrary.

## ops_from_json

```rust
pub fn ops_from_json(raw: &str, dur_ms: u32) -> Result<Vec<EditOp>, String>
```

Extracts and validates the AI plan from a raw model response string.

### Inputs

- `raw: &str` - the raw text returned by `ollama::chat`. May contain markdown fences (` ```json ... ``` `) around the JSON object. *Why:* despite the system prompt's instruction to return only JSON, models sometimes wrap output in fences; `extract_json` strips them transparently.
- `dur_ms: u32` - the clip's total duration in milliseconds. *Why:* used as the upper bound for all zoom and trim time values; zooms past the clip end are dropped rather than clamped to preserve the model's intent (a zoom near the very end is likely a mistake, not a rounding error).

### Implementation

1. Call `extract_json(raw)` to strip optional markdown fences and locate the outermost `{...}` object by brace-counting. Returns `None` (and thus `Err`) if no balanced object exists.
2. Deserialize the extracted slice into `Plan` using `serde_json::from_str`. `Plan.zooms` and `Plan.trim` both have `#[serde(default)]` so missing fields produce empty vec / `None` rather than a parse error.
3. Convert each `PlanZoom` to `EditOp::AddZoomFull`:
   - Clamp `scale` to `[1.0, 4.0]`; default to `2.0` if absent. *Why:* prevents extreme zooms that break layout or go below 1.0 (zoom-out is not supported).
   - Clamp `dur_ms` to `[200, clip_len]`. *Why 200 ms minimum:* sub-200 ms zooms are invisible at typical frame rates.
   - Use `checked_add` on `at_ms + zdur`; drop on overflow. *Why:* adversarial or buggy model output could produce `u32::MAX` durations that panic under debug overflow checks without this guard.
   - Drop any zoom whose `at_ms >= clip_len` or whose `end > clip_len`. *Why:* zooms past the clip boundary would reference non-existent frames.
4. If `Plan.trim` is present, validate and push `EditOp::SetTrim { in_ms, out_ms }`:
   - `in_ms = t.in_ms.min(t.out_ms)` guards against inverted trim values.
   - `out_ms = t.out_ms.min(clip_len)` guards against out-of-bound trim end.
   - Only pushed when `out_ms > in_ms`.
5. Return `Err("no usable edits")` if the ops vec is empty after all filtering. *Why error rather than Ok(empty):* callers use `?` and rely on the error to leave `edit.json` untouched.

### Returns

`Ok(Vec<EditOp>)` with at least one element on success. `Err(String)` if no JSON object is found in `raw`, if serde deserialization fails, or if all zooms are out-of-bounds and no trim is present.

### Behaviors

- `clean_json_yields_ops` - a well-formed JSON zoom produces one `AddZoomFull` with correct field values.
- `fenced_json_still_parses` - a response wrapped in ` ```json ``` ` fences is handled transparently.
- `scale_9_clamped_to_4` - a scale of `9.0` is clamped to `4.0`.
- `zoom_past_clip_is_dropped` - a zoom starting at 9900 ms with `dur_ms=500` in a 10000 ms clip is dropped, causing the function to return `Err`.
- `garbage_returns_err` - plain text with no JSON object returns `Err`.
- `valid_trim_maps_to_set_trim` - a valid `trim` block produces a `SetTrim` op with the correct `in_ms` and `out_ms`.
- `huge_dur_ms_is_dropped_not_overflow` - `dur_ms = u32::MAX` is caught by `checked_add` and dropped; the function returns `Err` rather than panicking on debug overflow checks.

### Used by

- `src-tauri/src/ai/commands.rs` - `ai_autoedit` calls `ops_from_json` after `ollama::chat` and applies the returned ops to the `EditDoc`.
