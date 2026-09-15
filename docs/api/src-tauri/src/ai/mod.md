# src-tauri/src/ai/mod.rs

MODULE OVERVIEW: the AI director pipeline. It turns the raw artifacts of a recording into something a local model can reason about, asks that model what to change, and returns a list of PROPOSALS the user reviews - it applies nothing itself.

Data flows in a straight line through three folders and one orchestrator: `plan::transcript` serialises the recording's logs into text, `frames::sample` picks the moments worth looking at and `frames::extract` pulls a JPEG for each from the preview proxy, `llm::vision` decides whether the chosen model can see them, `llm::prompt` supplies the matching system prompt, `llm::ollama` makes the one HTTP call, `plan::mapping` validates the reply into `EditOp`s that already exist, `plan::narrate` supplies a reason for anything the model left unexplained, `run` sequences all of it, and `commands` wraps `run` behind Tauri IPC. Only `run`, `commands`, `frames::extract` and the `llm` client touch the outside world; everything else is pure.

The `llm/` and `plan/` split (2026-09-15) was the first step of the M4 plan; `frames/` and `run.rs` are its perception and orchestration halves.

## commands

Tauri IPC surface: `ai_propose` (the propose pass, applying nothing), `list_ollama_models` (name plus vision flag, for the engine picker) and `pick_model`. The one-shot `ai_autoedit` command that used to apply-and-save the whole plan in one call was removed (bug-sweep-2 Task 7g): it had no frontend caller and blind-saved onto a pre-network-call doc snapshot, silently discarding any edits made during the LLM call.

## frames

Submodule folder (`ai/frames/`): which moments of the recording get a frame, and the JPEG for each. See `ai/frames/mod.md`.

## llm

Submodule folder (`ai/llm/`): the Ollama client, the system prompt, and the vision check. See `ai/llm/mod.md`.

## plan

Submodule folder (`ai/plan/`): the transcript, the schema, the validated mapping onto `EditOp`s, and the fallback reasons. See `ai/plan/mod.md`.

## run

Submodule (`ai/run.rs`): the whole propose pass, blocking. Full per-symbol docs in `ai/run.md`.
