# src-tauri/src/ai/llm/vision.rs

Whether a pulled Ollama model can read images. One `POST /api/show` per model name, cached for the app's life, and deliberately tolerant about what counts as a yes.

Being wrong here must degrade to the path that already works: a model wrongly called text-only still produces a plan from the transcript alone. So every failure - an unreachable server, a shape this code has never seen, unparseable JSON - resolves to `false`, and nothing in this file ever returns an error.

## has_vision

```rust
pub fn has_vision(model: &str) -> bool
```

### Inputs

- `model` - the Ollama model name exactly as `/api/tags` reported it.

### Returns

`true` when the model can accept `images` on a chat message.

### Implementation

A `OnceLock<Mutex<HashMap<String, bool>>>` memo, then `ask`. *Why cache:* the engine picker asks this for every installed model each time the AI panel mounts, and each miss is an HTTP round trip. *Why for the app's life rather than with an expiry:* a pulled model does not grow or lose a vision projector while the app is open, so an expiry would only buy repeated latency.

## ask

```rust
fn ask(model: &str) -> bool
```

`POST /api/show` with `{"model": ..., "name": ...}` (both keys, because the field was renamed across Ollama versions), a 2s connect and 10s read timeout, and `parse_show` on the body. Any error at all is `false`.

## parse_show

```rust
pub(crate) fn parse_show(json: &str) -> bool
```

Pure, so the decision is testable without a live Ollama. Three independent signals, any one of which is a claim of vision (A9):

1. `capabilities` is an ARRAY containing the string `"vision"` - what a current Ollama reports.
2. `details.families` names one of `VISION_FAMILIES` (`clip`, `mllama`, `llava`, `qwen2vl`, `qwen2_5vl`, `gemma3`, `siglip`) - an older server reports no capabilities array at all, so the family name is the only thing left to read.
3. `projector_info` is a NON-EMPTY object - older still.

### Behaviors

- `{"capabilities":"vision"}` is `false`: a string where an array belongs is not a claim of vision, it is a shape this code does not understand.
- `{"projector_info":{}}` is `false`. The key existing is not the claim; its contents are.
- Unparseable input is `false`, never an error.

## VISION_FAMILIES

```rust
const VISION_FAMILIES: [&str; 7]
```

Model families known to ship a vision projector, compared case-insensitively.
