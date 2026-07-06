# src-tauri/src/ai/backend/prompt.rs

Builds the static system prompt and declares the JSON output schema for the AI director. Pure and deterministic - `system_prompt()` always returns the same string for the same compiled binary, no I/O or config reads. The schema constant is kept separate so it can be embedded in the prompt body and referenced or tested independently.

## OUTPUT_SCHEMA

```rust
pub const OUTPUT_SCHEMA: &str =
    r#"{ "zooms": [ { "at_ms": <int>, "dur_ms": <int>, "scale": <float> } ], "trim": { "in_ms": <int>, "out_ms": <int> } }"#;
```

Template string describing the exact JSON shape the model must return. All time values are integers (milliseconds). `scale` is a float. `trim` is described as optional in the prompt rules - the model may omit it.

*Why a const:* ensures the schema shown to the model and any reference used in tests or downstream validation is always identical and single-source. Embedded verbatim in `system_prompt()` via the `{schema}` placeholder.

### Used by

- `src-tauri/src/ai/backend/prompt.rs` - interpolated into `system_prompt()` at the `{schema}` site.

## system_prompt

```rust
pub fn system_prompt() -> String
```

Returns the full system prompt sent to the LLM before the timeline transcript.

### Returns

A `String` containing the complete system prompt. It defines: the LLM's role as an "expert screen-recording editing director"; the transcript line format (clip header, click lines with region label, typing spans, idle spans, layout-change lines, text-field spans); seven numbered editing rules (zoom timing, scale range, no-overlap, no-zoom-during-idle, prefer fewer zooms, trim constraints, all values in milliseconds); the output JSON schema embedded via `OUTPUT_SCHEMA`; a concrete worked example; and a hard instruction to return only valid JSON with no prose or markdown fences.

### Implementation

1. Format the `r#"..."#` raw literal using `format!`, substituting `OUTPUT_SCHEMA` for `{schema}`.
2. Return the resulting `String`.

### Behaviors

- `prompt_is_non_empty` - the returned string is non-empty.
- `prompt_contains_json_keyword` - the prompt contains "json" (case-insensitive), confirming the output-format instruction is present.
- `prompt_contains_at_ms` - the prompt contains `"at_ms"`, confirming the output schema is embedded.

### Used by

- `src-tauri/src/ai/commands.rs` - `ai_autoedit` calls `system_prompt()` and passes the result as the `system` argument to `ai::ollama::chat`.
