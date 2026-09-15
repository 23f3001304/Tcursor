# src-tauri/src/ai/llm/mod.rs

The AI director's model side: the Ollama HTTP client, the system prompt it sends, and the capability check that decides whether frames go with it. Split out of the old `ai/backend/` (2026-09-15, the first step of the M4 plan) so the perception and plan halves of the director can grow in their own folders.

## ollama

Submodule (`ai/llm/ollama.rs`). The blocking Ollama client: model listing and chat, with or without images. Full per-symbol docs in `ai/llm/ollama.md`.

## prompt

Submodule (`ai/llm/prompt.rs`). The director's system prompt, in its seeing and its text-only form. Full per-symbol docs in `ai/llm/prompt.md`.

## vision

Submodule (`ai/llm/vision.rs`). Whether a pulled model can read images, cached per model. Full per-symbol docs in `ai/llm/vision.md`.
