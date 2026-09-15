# src-tauri/src/ai/frames/mod.rs

The director's eyes. Two halves, kept apart because one is pure and one shells out: `sample` decides WHICH moments of a recording are worth looking at, `extract` produces the JPEG for each of them.

Both work on the OUTPUT clock, the clock the preview proxy plays on and every `EditOp` is stored on, so a frame and the proposal that cites it always name the same instant.

## extract

Submodule (`ai/frames/extract.rs`). One JPEG per moment, pulled from the preview proxy through ffmpeg. Full per-symbol docs in `ai/frames/extract.md`.

## sample

Submodule (`ai/frames/sample.rs`). Pure: which moments of the clip get a frame, capped and spread. Full per-symbol docs in `ai/frames/sample.md`.
