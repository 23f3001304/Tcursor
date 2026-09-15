# src-tauri/src/ai/plan/narrate.rs

The words the review sheet shows when the model gave no reason of its own. Present tense, one clause, and for the kinds that have a place on screen it is correlated back to the click that happened there (reusing `transcript::region`, the same classifier the transcript itself uses) so the fallback still names the region the model was already shown.

This is a FALLBACK, not the headline. `mapping::tidy` keeps whatever reason the model wrote; `ai::run::propose` calls into this file only for a proposal whose `why` came back empty.

## why_for

```rust
pub fn why_for(kind: ProposalKind, at_ms: u32, log: &EventLog, shift: i64) -> String
```

### Inputs

- `kind` - what is being proposed. The reason is about the MOMENT, so the kind only decides which question is being answered: why look closer, why cut, why speed up.
- `at_ms` - the proposal's start on the OUTPUT clock.
- `log` - the recording's mouse events; supplies both the click list and `log.screen` for the origin conversion and `region()`.
- `shift` - ms to ADD to `log`'s raw EVENT-clock timestamps to land on the OUTPUT clock (`edit::seed::output_shift`, the same value that built the transcript). `at_ms` is already on that clock. Without shifting the clicks too, an output-clock `at_ms` would be compared against raw event-clock click times, off by the capture-warmup lead (around 800ms on a real recording), which is enough to miss the correlating click entirely or to attribute an unrelated one.

### Returns

One line, ready to display. Never empty, never multi-line.

### Implementation

`near_click` first, then a per-kind sentence: a zoom or a spotlight names the region when there is a click to name it from and falls back to the time when there is not; a layout switch says the person turned to the camera; a trim names the clip's ends; a cut and a speed-up name the time. *Why the fallbacks still carry a time:* "a moment worth a closer look" with no anchor is not a reason, it is filler.

### Behaviors

- A zoom 100ms after a top-left click reads `"you click the top-left there"`.
- A click at raw event `t=3100` with `shift=-800` is at output-clock 2300ms, so a proposal at `at_ms=2300` finds it. Comparing raw `e.t` against `at_ms` (the pre-fix bug) missed it by 800ms.
- The same fixture with a secondary monitor at `origin_x=1920`: a raw click at `(2880,540)` is screen-local `(960,540)`, so it reads as `"center"`. Both the one-clock and the screen-origin fixes have to compose for the region to be right.
- Every kind returns something honest and single-line, including the ones with no click anywhere near them.

## near_click

```rust
fn near_click(at_ms: u32, log: &EventLog, shift: i64) -> Option<&'static str>
```

The screen region of the nearest `EventKind::Down` within 600ms of `at_ms`, both on the output clock. *Why 600ms:* wide enough to survive the prompt's own "start up to 0.3s before the click" rule plus clock rounding, narrow enough that an unrelated click a second away is not claimed as the cause. The click's raw virtual-desktop coordinates go through `coordmap::to_frame` before `region()` sees them, so the reason always agrees with what the transcript showed the model.

## mmss

```rust
fn mmss(ms: u32) -> String
```

`"<minutes>:<seconds>"`, seconds zero-padded (`68_000` gives `"1:08"`).
