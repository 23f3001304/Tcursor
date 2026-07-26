# src-tauri/src/ai/backend/ollama.rs

Thin synchronous HTTP client for a local Ollama instance: `chat` posts a two-message chat request and returns the model's reply as a raw string; `list_models` queries which models are installed. This is the only file in the codebase that makes an outbound network call; `chat`'s 180-second read timeout is intentional to accommodate slow local inference on a laptop GPU or CPU.

## list_models

```rust
pub fn list_models() -> Vec<String>
```

Lists locally-installed Ollama model names via `GET http://localhost:11434/api/tags`.

### Returns

`Vec<String>` of model names (e.g. `["llama3.2", "mistral"]`), or an empty `Vec` - never an `Err` - if the request fails (Ollama not running) or the response doesn't parse as the expected `{"models":[{"name":...}]}` shape. *Why swallow errors here but not in `chat`:* this powers an optional UI convenience (the Engine picker's dropdown); `ai_autoedit` must still work via `chat`'s own default model name when Ollama is up but this call fails for an unrelated reason.

### Implementation

Builds a short-timeout (2s connect) `ureq::Agent` - deliberately much shorter than `chat`'s, since this call gates how quickly the AI panel becomes interactive, not the (already-consented-to) auto-edit run. `.get(...).call()` failure or a failed `.into_json::<TagsResp>()` both fall through to an empty `Vec` via `unwrap_or_default()`.

### Used by

- `src-tauri/src/ai/commands.rs` (`list_ollama_models`) - the only caller, a direct passthrough over Tauri IPC.

## chat

```rust
pub fn chat(model: &str, system: &str, user: &str) -> Result<String, String>
```

Sends a chat completion request to `http://localhost:11434/api/chat` and returns the model's text content.

### Inputs

- `model: &str` - the Ollama model name, passed through verbatim (e.g. `"llama3.2"`). *Why caller-supplied:* the model choice belongs to the user's settings, not to the HTTP layer; keeping the client model-agnostic means switching models requires no code change.
- `system: &str` - the system message content, produced by `ai::prompt::system_prompt()`. *Why separate from user:* Ollama's chat API uses role-based messages; passing them separately here keeps the prompt construction in `prompt.rs` rather than scattered across the client.
- `user: &str` - the user message content, the serialized timeline transcript from `ai::timeline::serialize`. *Why the whole transcript as one message:* the model sees the full recording context in a single turn; multi-turn is unnecessary for a batch editing task.

### Implementation

1. Build a `ChatReq` struct with `model`, two `Msg` values (`role: "system"` and `role: "user"`), `stream: false`, and `format: "json"`. *Why `stream: false`:* the caller needs the complete response before parsing; streaming would require a stateful consumer with no benefit here. *Why `format: "json"`:* instructs Ollama to constrain its sampler to produce valid JSON output, reducing parse failures in `ai::plan`.
2. Serialize to `serde_json::Value` via `to_value`. *Why `Value` rather than `to_string`:* `ureq`'s `send_json` accepts `Value` and handles `Content-Type` automatically.
3. Build a `ureq::Agent` with connect timeout 10 s and read timeout 180 s. *Why 10 s connect:* if Ollama is not running the error should surface quickly. *Why 180 s read:* a 7B-parameter model on a mid-range laptop can take over a minute to generate a full JSON response; the timeout must not cut it short.
4. POST to `http://localhost:11434/api/chat` with `Content-Type: application/json`. Map `ureq::Error::Transport` to a human-readable message including `"ollama serve"` so users know exactly what to run. Map `ureq::Error::Status` to `"Ollama returned HTTP <code>"`.
5. Deserialize the response body into `ChatResp` (which has `message: RespMsg { content: String }`). Map deserialization errors to `"could not parse Ollama response: ..."`.
6. Return `Ok(parsed.message.content)`.

### Returns

`Ok(String)` containing the model's raw reply (expected to be a JSON object, but not validated here - validation is `ai::plan`'s responsibility). `Err(String)` with a human-readable message on any of: request serialization failure, transport error, non-2xx HTTP status, or response deserialization failure.

### Used by

- `src-tauri/src/ai/commands.rs` - `ai_autoedit` calls `chat` after building the transcript and passes the raw response to `ai::plan::ops_from_json`.
