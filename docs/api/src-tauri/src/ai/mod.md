# src-tauri/src/ai/mod.rs

MODULE OVERVIEW: The `ai` module implements the AI director pipeline: it converts raw recording artifacts into a plain-text timeline transcript, sends that transcript to a local Ollama LLM, parses the model's JSON reply into validated edit operations, and writes the result back to `edit.json`. Data flows in a straight line through five submodules: `timeline` serializes recording logs into text, `prompt` supplies the static system prompt, `ollama` makes the HTTP call, `plan` validates and converts the raw reply into safe `EditOp` values, and `commands` orchestrates the whole sequence as a single Tauri IPC command. Only `commands` and `ollama` perform I/O; all other submodules are pure functions.

## timeline

Converts mouse events, hotkey actions, cursor-type samples, and typing timestamps into a compact plain-text transcript for the LLM, capped at 120 moments to control token usage. Key items: `serialize` (single entry point; returns the full transcript string from all recording data sources).

## prompt

Builds the static system prompt and declares the JSON output schema sent to the LLM before the timeline transcript; pure and deterministic with no I/O. Key items: `system_prompt` (returns the complete system prompt string including rules and worked example), `OUTPUT_SCHEMA` (const JSON shape string embedded in the prompt and usable for validation).

## plan

Parses and validates the LLM's raw chat reply into safe, clamped `EditOp` values; acts as the trust boundary between untrusted model output and the edit document. Key items: `ops_from_json` (extracts JSON from the reply, strips markdown fences, clamps numeric fields to clip bounds, returns `Vec<EditOp>` or an error).

## ollama

Thin synchronous HTTP client that posts a two-message chat request to a local Ollama instance and returns the model's raw reply string; the only file in the `ai` module that makes an outbound network call. Key items: `chat` (POSTs to `http://localhost:11434/api/chat` with a 180-second read timeout and returns the model's text content).

## commands

Single Tauri IPC command that orchestrates the full AI auto-edit pipeline: loads session artifacts, builds the transcript, calls Ollama, parses the reply, and writes the updated `EditDoc`. Key items: `ai_autoedit` (Tauri command; loads event log, action log, cursor track, and typing log; calls `timeline::serialize`, `ollama::chat`, and `plan::ops_from_json`; saves the result to `edit.json`).
