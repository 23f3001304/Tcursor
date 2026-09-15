# src-tauri/src/ai/run.rs

The propose pass end to end: load the recording, describe it, look at it, ask the model, validate what comes back. Everything blocking lives here so `ai::commands` stays a thin `spawn_blocking` wrapper.

One clock runs through the whole file. `edit::seed::output_shift` is read ONCE and passed to the transcript, the frame times, the click points and the fallback reasons, so none of them can disagree about when something happened.

## propose

```rust
pub fn propose(paths: &ProjectPaths, model: Option<String>) -> Result<AiRun, String>
```

### Inputs

- `paths` - the project folder.
- `model` - the user's picked engine, or `None` to let `pick_model` choose an installed one.

### Returns

`Ok(AiRun)` even when the model proposed nothing: an empty sheet is an answer. `Err` only for things the user can act on - no models installed, an unreadable event log, Ollama unreachable, the model not pulled - and the message says which.

### Implementation

1. `load_or_seed`, then the TRUE duration from `true_duration_ms`, falling back to `doc.trim.out_ms`. *Why not `doc.trim.out_ms` first:* it is `0` after a trim reset (`0` means "whole clip"), which would feed the director a zero-length timeline.
2. Load the event log, action log, cursor track and typing log, and `transcript::serialize` them on the output clock.
3. `list_models` and `pick_model`, then `has_vision` on the chosen one.
4. With vision: `sample_times` over the click and layout times, then `jpegs_at`, then `base64_encode` per frame (`export::preview::base64_encode`, the same RFC 4648 encoder every data-URL command in the app uses - no `base64` crate is added, A7). Without vision: no frames at all, and the prompt never mentions any.
5. `chat_with_images`, `proposals_from_json`, then `why_for` for any proposal the model left unexplained.
6. `frames: images.len()` - what was actually received, NOT what was asked for. A frame that failed to extract was never seen, and the sheet header must not claim it.

### Behaviors

- A text-only model takes exactly the path it took before vision existed: no proxy is built, no ffmpeg runs, and the request body is byte-identical to the old one.
- A proxy that cannot be built yields no frames rather than an error, and the run continues text-only.

## click_points

```rust
fn click_points(log: &EventLog, shift: i64) -> Vec<ClickAt>
```

Every `EventKind::Down` on the output clock, its point divided by the screen size into the 0..1 fractions `ZoomTarget::Fixed` stores. The raw positions are virtual-DESKTOP coordinates, so `coordmap::to_frame` subtracts the screen origin first - the same conversion the transcript and the narration apply, which is why a zoom aimed here lands where the transcript said the click was. Clamped into `[0, 1]`, since a click recorded during a mid-take display switch can fall outside the new screen.

## layout_times

```rust
fn layout_times(actions: &[ActionEvent], shift: i64) -> Vec<u32>
```

`ActionKind::SetLayout` times on the output clock. Every other recorded action is a hold, which the seed has already turned into a region and which needs no frame of its own.
