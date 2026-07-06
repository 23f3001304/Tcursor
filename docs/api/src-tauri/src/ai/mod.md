# src-tauri/src/ai/mod.rs

MODULE OVERVIEW: The `ai` module implements the AI director pipeline: it converts raw recording artifacts into a plain-text timeline transcript, sends that transcript to a local Ollama LLM, parses the model's JSON reply into validated edit operations, and writes the result back to `edit.json`. Data flows in a straight line through five submodules: `timeline` serializes recording logs into text, `prompt` supplies the static system prompt, `ollama` makes the HTTP call, `plan` validates and converts the raw reply into safe `EditOp` values, and `commands` orchestrates the whole sequence as a single Tauri IPC command. Only `commands` and `ollama` perform I/O; all other submodules are pure functions.

## commands

Single Tauri IPC command that orchestrates the full AI auto-edit pipeline: loads session artifacts, builds the transcript, calls Ollama, parses the reply, and writes the updated `EditDoc`. Key items: `ai_autoedit` (Tauri command; loads event log, action log, cursor track, and typing log; calls `timeline::serialize`, `ollama::chat`, and `plan::ops_from_json`; saves the result to `edit.json`).
