# src-tauri/src/ai/plan/mapping.rs

The model's reply turned into proposals, and the only place that decides what the director is allowed to suggest. Every proposal maps onto `EditOp`s that already exist; this milestone adds no op and no `EditDoc` field.

The governing rule is that a bad item costs its siblings nothing. Parsing is per element (`Vec<serde_json::Value>`, then `from_value::<RawEdit>` on each), and every validation below drops ONE item, never the run. An unreadable reply is an empty sheet, which the sheet states in words, rather than an error dialog.

## proposals_from_json

```rust
pub fn proposals_from_json(raw: &str, dur_ms: u32, clicks: &[ClickAt]) -> Vec<AiProposal>
```

### Inputs

- `raw` - the model's reply, fences and prose and all.
- `dur_ms` - the clip's TRUE length, the bound every time is checked against.
- `clicks` - every click on the output clock, for snapping (`ai::run::click_points`).

### Implementation

`extract_json`, then `PlanV2 { edits: Vec<Value> }`, then `one` per element, then a STABLE sort by `at_ms` (so an overlap always loses to the earlier item), then `drop_overlaps`, then ids `p0..pn` in that final order.

### Behaviors

- One item with a string where `at_ms` belongs is dropped and its siblings survive. A typed `Vec<RawEdit>` would have failed the whole array on it, which binding decision 2 forbids outright.
- Garbage, an empty `edits` array and an empty string all return an empty vec.

## one

```rust
fn one(e: &RawEdit, total: u32, clicks: &[ClickAt]) -> Option<AiProposal>
```

One item validated and mapped, in this order: unknown kind drops; a kind outside `RENDERABLE_KINDS` drops; `why` is tidied; the rect is kept only if `sane_rect`; a trim goes to `trim`; everything else gets a `span`, then its own rule.

- **Zoom.** `snap_to_click` may move the start onto a real click; `scale = (1 / rect_long_side).clamp(MIN_SCALE, MAX_SCALE)`, or `2.0` with no rect. Ops are `[AddZoomFull]` plus, when there is a point to aim at, an `UpdateZoom { id: NEW_ID, target: Fixed }`. The model's own rect beats the click's point (it named the thing; the click only says where the pointer was), but the snapped TIME always wins over the model's guess.
- **Layout.** `valid_layout`, then only `presenter` / `camera` / `camera_only` survive. A switch back to the plain screen is where the recording already is: a row that reads as an edit and changes nothing.
- **Spotlight.** DROPPED without a rect. The spotlight renders at the CURSOR (`export/fx/fx_state.rs` centres it on `project(cur.x, cur.y, ..)`), so with no region there is no way to know it will land where the model meant, and proposing it anyway is the confidently wrong answer this milestone exists to remove (A4). With one, `radius = (long_side / 2).clamp(0.05, 0.40)` on an `UpdateEffect { id: NEW_ID }`.
- **Cut.** `AddCuts { spans: [(at, at + dur)] }` - the batch op, so a row is one undo step whether it carries one span or many.
- **Speed.** Dropped without a usable `factor` (finite, and more than 1% away from 1.0, which is no change at all). Clamped to the remap's own `0.25..8.0`.

### Behaviors

- A rect outside the frame leaves a cursor-following zoom rather than dropping it: the moment is still worth proposing, but `rect` is cleared so the sheet cannot outline a region this file refused.
- A zoom within a second of a click starts at `click - PRE_ROLL_MS` and borrows the click's point. Beyond a second, the model's own timing stands and the zoom follows the cursor.

## span

```rust
fn span(kind: ProposalKind, at: u32, raw_dur: u32, total: u32) -> Option<(u32, u32)>
```

`(start, duration)` if the item fits in the clip at all. The RAW duration decides that, BEFORE any clamp: clamping first would turn an adversarial `u32::MAX` into a comfortable 6s and hide the fact that the model invented it. `checked_add` throughout, so an adversarial duration drops the item instead of overflowing. Then the clamp: zoom and spotlight to `[600, 6000]`, layout and speed to `[1000, total]`, everything else to `[300, total]`, each bound itself clamped to `total` so a sub-second clip cannot invert the range.

## trim

```rust
fn trim(e: &RawEdit, total: u32, why: String) -> Option<AiProposal>
```

The 0-sentinel rules, verbatim from the v1 plan: `out_ms == 0` means "runs to the true end", NOT "zero length", so a head-only `{2000, 0}` must keep its zero rather than collapse through `min(in, out)` into an empty range. `{0, 0}` is no decision at all and produces nothing. The proposal sits at `at_ms: 0, dur_ms: 0`: a trim is a property of the clip's ends, not a moment in it.

## snap_to_click

```rust
pub(crate) fn snap_to_click(at_ms: u32, clicks: &[ClickAt]) -> Option<(u32, f32, f32)>
```

The nearest click within `SNAP_MS`, returned as `(t - PRE_ROLL_MS, x, y)` with the pre-roll already subtracted, saturating at 0.

## sane_rect

```rust
fn sane_rect(r: &[f32; 4]) -> bool
```

All four finite, `w > 0.02`, `h > 0.02`, and inside `[0, 1]` with a 0.001 tolerance for a model that rounds. A rect smaller than 2% of the frame is a point, not a region, and aiming a zoom at it would magnify the model's own error.

## tidy

```rust
fn tidy(why: &str) -> String
```

Whitespace collapsed to single spaces, then cut at a word boundary to `WHY_MAX`. A single word longer than the cap falls back to a hard character truncation rather than returning nothing. An empty reason stays empty, and `ai::run` fills it from `narrate::why_for` instead of shipping a row with no reason on it.

## drop_overlaps

```rust
fn drop_overlaps(sorted: Vec<AiProposal>) -> Vec<AiProposal>
```

Within a kind, a later item overlapping one already kept is dropped. A zero-length item (a trim) still occupies its instant (`dur_ms.max(1)`), so a second trim cannot sit on top of the first and quietly win by being applied last.

## PlanV2

```rust
struct PlanV2 { edits: Vec<serde_json::Value> }
```

Untyped elements on purpose: see the per-item resilience note at the top.

## RawEdit

```rust
struct RawEdit { kind, at_ms, dur_ms, rect, layout, factor, in_ms, out_ms, why }
```

The union of every field any kind uses, all defaulted. One struct rather than a tagged enum so a model that sends a `rect` on a trim, or omits a field entirely, still produces a parseable item that the per-kind rules can then judge.
