# src-tauri/src/ai/backend/mod.rs

Submodule overviews for the `backend` group.

## ollama

Thin synchronous HTTP client that posts a two-message chat request to a local Ollama instance and returns the model's raw reply string; the only file in the `ai` module that makes an outbound network call. Key items: `chat` (POSTs to `http://localhost:11434/api/chat` with a 180-second read timeout and returns the model's text content).

## prompt

Builds the static system prompt and declares the JSON output schema sent to the LLM before the timeline transcript; pure and deterministic with no I/O. Key items: `system_prompt` (returns the complete system prompt string including rules and worked example), `OUTPUT_SCHEMA` (const JSON shape string embedded in the prompt and usable for validation).

## plan

Parses and validates the LLM's raw chat reply into safe, clamped `EditOp` values; acts as the trust boundary between untrusted model output and the edit document. Key items: `ops_from_json` (extracts JSON from the reply, strips markdown fences, clamps numeric fields to clip bounds, returns `Vec<EditOp>` or an error).

## timeline

Converts mouse events, hotkey actions, cursor-type samples, and typing timestamps into a compact plain-text transcript for the LLM, capped at 120 moments to control token usage. Key items: `serialize` (single entry point; returns the full transcript string from all recording data sources).
