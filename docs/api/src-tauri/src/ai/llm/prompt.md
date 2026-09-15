# src-tauri/src/ai/llm/prompt.rs

The system prompt the director sends, and the output schema it asks for. Pure and deterministic: the same string every time for the same compiled binary, no I/O and no config reads. The schema is a separate constant so the prompt body and any test reference the one source.

## OUTPUT_SCHEMA

```rust
pub const OUTPUT_SCHEMA: &str
```

The v2 wire shape: one object, one `edits` array, one line per kind the director understands (`zoom`, `layout`, `spotlight`, `cut`, `speed`, `trim`) showing exactly which fields that kind takes. All times are integer milliseconds; `rect` is `[x, y, w, h]` in 0..1 fractions of the frame.

*Why a const:* the schema shown to the model and the one `plan::mapping` parses must be a single source, or the prompt drifts from the parser and every failure looks like the model's fault.

### Used by

- `src-tauri/src/ai/llm/prompt.rs` - interpolated into `system_prompt` at the `{schema}` site.

## system_prompt

```rust
pub fn system_prompt(vision: bool) -> String
```

### Inputs

- `vision` - whether this run is sending frames. `true` inserts the paragraph telling the model it is receiving still frames in time order, each labelled with its time in the transcript, and that a `rect` should name only something it can actually see. A text-only model gets the identical rules WITHOUT that paragraph: promising images that are not there is how a model starts inventing regions.

### Returns

The full prompt: the role, the transcript line format, the goal (with permission to propose nothing), nine numbered rules, the schema, one worked example, and the instruction to return only JSON.

### Implementation

One `format!` over a raw literal with two substitutions, `{eyes}` and `{schema}`.

### Behaviors

- `the_prompt_names_the_schema_it_wants_back` - the prompt mentions JSON, `at_ms` and `"why"`.
- `a_text_only_model_is_never_told_about_frames_it_cannot_see` - the "still frames" paragraph appears only under `vision: true`.
- `every_offered_kind_is_asked_for_by_name` - all six `RENDERABLE_KINDS` are named in the prompt, so nothing the mapping accepts is a kind the model was never asked for.

### Used by

- `src-tauri/src/ai/run.rs` - `propose` calls `system_prompt(vision)` with the result of `llm::vision::has_vision` and passes it as the `system` argument to `ollama::chat_with_images`.
