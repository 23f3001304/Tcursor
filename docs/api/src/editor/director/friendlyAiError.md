# src/editor/director/friendlyAiError.ts

Error-copy mapping for the AI Director panel.

## friendlyAiError

```ts
export function friendlyAiError(raw: string): { title: string; hint: string | null }
```

Turns a raw AI-director failure string (`String(e)` off the `ai_plan` IPC rejection) into a short title + an actionable hint, for the two failure modes an Ollama setup actually hits day to day. Unit-tested against the EXACT wording the backend produces (`friendlyAiError.test.ts`).

### Behavior

- Contains `"No Ollama models"` (matches `build_plan`'s `"No Ollama models are installed. Pull one first, e.g.: ollama pull llama3.2"`, `src-tauri/src/ai/commands.rs`) -> `{ title: "No local model installed", hint: "Run: ollama pull llama3.2" }`.
- Matches `/could not reach ollama|is it running|refused|timed? ?out/i` (covers `ollama::chat`'s `"Could not reach Ollama at localhost:11434. Is it running? Try: ollama serve"`, `src-tauri/src/ai/backend/ollama.rs`, plus generic connection-refused/timeout wording) -> `{ title: "Ollama isn't running", hint: "Start the Ollama app, then try again" }`.
- Anything else -> `{ title: raw, hint: null }` - the raw message passes through unchanged, same as before this mapping existed; `AiPanel` renders the hint line only when `hint` is non-null.
