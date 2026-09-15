# src-tauri/src/ai/plan/json.rs

Finding the JSON object inside whatever a model actually replied with. One function, moved out of the v1 `plan.rs` unchanged when the v2 schema replaced it, because the problem it solves is about models rather than about any particular schema.

## extract_json

```rust
pub fn extract_json(raw: &str) -> Option<&str>
```

### Returns

A slice of `raw` covering exactly one balanced `{...}`, or `None` when there is no complete object in it.

### Implementation

1. Strip a markdown fence if there is one: from the first ``` to the end of that line, up to the next ``` (or the end). *Why bother when the request sets `format: "json"`:* that option constrains the reply to be VALID JSON, not to be nothing but JSON, and a model that has been told to explain itself will happily fence its answer.
2. Balance-count braces from the first `{`, STRING-AWARE. A `{` or `}` inside a string value is not structure and must not move the depth counter. The scanner tracks whether it is inside a string, toggling on an unescaped `"` and skipping the character right after a `\` so an escaped quote cannot end the string early.
3. `None` if the depth never returns to zero.

### Behaviors

- `{"why":"the idle stretch } at the start", ...}` returns the WHOLE object. Without string-awareness the brace inside the reason would be taken for the closing one and the plan would be truncated to nothing.
- The mirror case, an unmatched `{` inside a string, must not imbalance the depth either: without string-awareness it would never return to zero and a perfectly valid reply would be reported as no JSON at all.
- An unterminated object (`{"edits":[`) is `None`, not a partial parse.
