# src/editor/director/engineName.ts

Short display-name derivation for Ollama model ids.

## engineDisplayName

```ts
export function engineDisplayName(id: string): string
```

Ollama model ids can be very long, especially HF GGUF proxies like `"hf.co/empero-ai/Qwythos-9B-Claude-Mythos-5-1M-GGUF:Q8_0"` (ux audit #16/#17: this wrapped over two lines in `AiPanel`'s Engine dropdown, and printed in full on `DirectorScrim`'s running-status pill). Derives a short `"Family Size (Quant)"` label - e.g. `"Qwythos 9B (Q8)"` - for display; callers keep the raw id as the real value and put it in a `title` attribute so it's still there on hover, not lost.

Pure/best-effort: drops a host/org prefix (the last `/`-segment is the model name), splits the Ollama `name:tag` convention and scans BOTH halves for a size token (`/^\d+(\.\d+)?[bmk]$/i`, e.g. `"9b"`, `"1.5B"`) and a quant token (`/^q\d/i`, e.g. `"q4"`, `"Q8_0"`'s leading `"Q8"`) - either side can carry either piece of info depending on how the id was built (a bare HF GGUF repo name has size baked into the name; a plain Ollama pull like `"llama3.1:8b-instruct-q4_0"` has it in the tag instead).

### Behavior

- A size token is found -> `"{family} {SIZE}"` (uppercased), e.g. `"Qwythos 9B"`, `"llama3.1 8B"`.
- No size token -> the whole (GGUF-suffix-stripped) name is kept AS-IS rather than collapsed to just the first word - e.g. `"Phi-3-Mini"` stays `"Phi-3-Mini"`, not `"Phi"`, since the rest of a hyphenated name might be all that distinguishes two installed variants (`"Phi-3-Mini"` vs `"Phi-3-Medium"`).
- A quant token is found (either path above) -> appended as `" (QUANT)"`, uppercased.
- Empty/whitespace input -> passed through unchanged (never throws, never returns a non-empty input as `""`).

Unit-tested (`engineName.test.ts`) against the audit's exact example id, a plain-Ollama tag-half id, a no-size/no-quant fallback, a host/org-prefix strip, and an empty input.

### Used by

- `AiPanel` - the Engine `Picker`'s option `label`s (`title` carries the full raw id).
- `DirectorScrim` - the running-status pill's copy (`shortModel`).
