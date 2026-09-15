# src-tauri/src/ai/plan/mod.rs

The AI director's plan side: the transcript of the recording it reads, the JSON it digs out of a reply, the schema that reply must fit, the ops that reply becomes, and the words it explains itself with. Split out of the old `ai/backend/` (2026-09-15, the first step of the M4 plan); `timeline.rs` became `transcript.rs` in the move, since a "timeline" in this app is the editor's lane stack and this file serialises the recording's events.

The v1 planner (a zooms-and-trim schema returning a bare `Vec<EditOp>`) was replaced by `schema` + `mapping` when the director started proposing rather than applying; its brace-balancing parser survives unchanged in `json`.

## json

Submodule (`ai/plan/json.rs`). Finding the JSON object inside whatever the model actually replied with. Full per-symbol docs in `ai/plan/json.md`.

## mapping

Submodule (`ai/plan/mapping.rs`). Validation and the map onto existing `EditOp`s, per item. Full per-symbol docs in `ai/plan/mapping.md`.

## narrate

Submodule (`ai/plan/narrate.rs`). The fallback reason for a proposal the model left unexplained. Full per-symbol docs in `ai/plan/narrate.md`.

## schema

Submodule (`ai/plan/schema.rs`). The shape of a run and the constants it is validated against. Full per-symbol docs in `ai/plan/schema.md`.

## transcript

Submodule (`ai/plan/transcript.rs`). The recording's events serialised as the text the model reads. Full per-symbol docs in `ai/plan/transcript.md`.
