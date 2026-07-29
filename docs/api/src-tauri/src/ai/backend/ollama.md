# src-tauri/src/ai/backend/ollama.rs

Thin synchronous HTTP client for a local Ollama instance: `chat` posts a two-message chat request and returns the model's reply as a raw string; `list_models` queries which **chat** models are installed, filtering out embedding-only ones. This is the only file in the codebase that makes an outbound network call; `chat`'s 180-second read timeout is intentional to accommodate slow local inference on a laptop GPU or CPU.

## list_models

```rust
pub fn list_models() -> Vec<String>
```

Lists locally-installed Ollama **chat** model names via `GET http://localhost:11434/api/tags`, filtering out embedding-only models.

### Returns

`Vec<String>` of chat model names (e.g. `["llama3.2", "mistral"]`), or an empty `Vec` - never an `Err` - if the request fails (Ollama not running), the response doesn't parse as the expected `{"models":[{"name":...}]}` shape, or every installed model is embedding-only. *Why filter embeddings:* models whose name contains `"embed"` (case-insensitive - catches `nomic-embed-text`, anything `*-embed-*`) don't support `/api/chat`; offering one in the Engine picker or auto-picking it as the default model would just 404. *Why swallow errors here but not in `chat`:* this powers an optional UI convenience (the Engine picker's dropdown) and `build_plan`'s default-model lookup; both tolerate an empty list (the picker falls back to showing the current/default name, `build_plan` surfaces its own "no models installed" error only if the caller also gave no explicit pick).

### Implementation

Builds a short-timeout (2s connect) `ureq::Agent` - deliberately much shorter than `chat`'s, since this call gates how quickly the AI panel becomes interactive, not the (already-consented-to) auto-edit run. `.get(...).call()` failure or a failed `.into_json::<TagsResp>()` both fall through to an empty `Vec` via `unwrap_or_default()`. On success, maps each entry to its `name` and drops any whose lowercased name contains `"embed"` before collecting - this is the "pick a chat model, not an embedder" filter.

### Used by

- `src-tauri/src/ai/commands.rs` (`list_ollama_models`) - direct passthrough over Tauri IPC, populating `AiPanel.tsx`'s Engine picker.
- `src-tauri/src/ai/commands.rs` (`build_plan`) - the shared LLM pass behind both `ai_autoedit` and `ai_plan` calls this to resolve the actually-installed model: the caller's pick if it's in the list, else the first installed chat model, else an error (no hardcoded model name is ever used).

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
4. POST to `http://localhost:11434/api/chat` with `Content-Type: application/json`. Map `ureq::Error::Transport` to a human-readable message including `"ollama serve"` so users know exactly what to run. Map `ureq::Error::Status(404, _)` specifically to `"Model '<model>' isn't installed in Ollama. Pull it (ollama pull <model>) or pick an installed model."` - *why call out 404:* a 404 here means the named model isn't pulled; naming the model and the fix inline turns what would otherwise be an opaque `"Ollama returned HTTP 404"` into an actionable message. Map any other non-2xx status to `"Ollama returned HTTP <code>"`.
5. Deserialize the response body into `ChatResp` (which has `message: RespMsg { content: String }`). Map deserialization errors to `"could not parse Ollama response: ..."`.
6. Return `Ok(parsed.message.content)`.

### Returns

`Ok(String)` containing the model's raw reply (expected to be a JSON object, but not validated here - validation is `ai::plan`'s responsibility). `Err(String)` with a human-readable message on any of: request serialization failure, transport error, a 404 naming the missing model, any other non-2xx HTTP status, or response deserialization failure.

### Used by

- `src-tauri/src/ai/commands.rs` - `build_plan` (the shared LLM pass behind both `ai_autoedit` and `ai_plan`) calls `chat` after building the transcript and resolving an installed model name, and passes the raw response to `ai::backend::plan::ops_from_json`.
