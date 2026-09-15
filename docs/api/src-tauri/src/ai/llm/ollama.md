# src-tauri/src/ai/llm/ollama.rs

Thin synchronous HTTP client for a local Ollama instance: `chat` and `chat_with_images` post one chat turn and return the model's reply as a raw string; `list_models` says which chat models are installed. This and `vision.rs` are the only files in the codebase that make outbound network calls. The long read timeouts are deliberate: local inference on a laptop GPU or CPU is slow, and cutting it short would report a timeout for work that was about to succeed.

## OllamaModel

```rust
pub struct OllamaModel { pub name: String, pub vision: bool }
```

One installed model as the engine picker sees it. `vision` comes from `llm::vision::has_vision`, not from the name: an id says nothing reliable about whether a projector was packed with it.

## list_models

```rust
pub fn list_models() -> Vec<String>
```

Locally-installed CHAT model names via `GET http://localhost:11434/api/tags`.

### Returns

Chat model names, or an empty `Vec` - never an `Err` - when Ollama is unreachable, the response does not parse, or every installed model is embedding-only.

### Implementation

A 2s connect timeout, deliberately much shorter than a chat's: this call gates how quickly the AI panel becomes interactive, not an already-consented-to run. Models whose lowercased name contains `"embed"` are dropped (`nomic-embed-text`, anything `*-embed-*`): they do not support `/api/chat`, so offering one in the picker or auto-picking it as the default would just 404.

### Used by

- `src-tauri/src/ai/commands.rs` (`list_ollama_models`) - each name paired with `has_vision` for the Engine picker.
- `src-tauri/src/ai/run.rs` (`propose`) - the candidate list `pick_model` resolves the user's choice against.

## chat_body

```rust
pub(crate) fn chat_body<'a>(model: &'a str, system: &'a str, user: &'a str, images: &'a [String]) -> ChatReq<'a>
```

The request body, separated from the call so what goes on the wire is testable without a live Ollama. Two messages, `stream: false` (the caller needs the whole reply before parsing) and `format: "json"` (constrains the sampler to valid JSON, which is what makes `plan::json`'s brace scanner reliable).

### Behaviors

- `images_ride_on_the_user_message_not_on_the_request` - `images` is a field of the USER MESSAGE, not of the request and not of the system message (A8). Ollama ignores a top-level `images`, so getting this wrong would look exactly like a model that cannot see.
- `a_text_only_request_is_byte_identical_to_the_old_one` - an empty `images` emits no `images` key at all, not an empty array (`skip_serializing_if`), so a text-only model receives exactly what it received before vision existed.

## chat_with_images

```rust
pub fn chat_with_images(model: &str, system: &str, user: &str, images: &[String]) -> Result<String, String>
```

One chat turn with `images` (base64, no data-URL prefix) attached to the user message.

### Implementation

`chat_body`, then a `ureq::Agent` with a 10s connect timeout and a read timeout of 180s text-only or 600s with images. *Why 600:* a 16-image pass on a local vision model routinely runs past three minutes. `ureq::Error::Transport` becomes the "is Ollama running? try ollama serve" message; a 404 names the model and the `ollama pull` that fixes it; any other status reports its code.

### Returns

`Ok(String)` with the model's raw reply, not validated here - that is `plan::mapping`'s job. `Err(String)` with a message a user can act on.

## chat

```rust
pub fn chat(model: &str, system: &str, user: &str) -> Result<String, String>
```

`chat_with_images` with no images: exactly what this client did before vision existed.

## Msg

```rust
struct Msg<'a> { role, content, images: Option<&'a [String]> }
```

One chat message. `images` is skipped when `None`, and only the user message ever carries it.

## ChatReq

```rust
struct ChatReq<'a> { model, messages, stream, format }
```

The `/api/chat` request body.
