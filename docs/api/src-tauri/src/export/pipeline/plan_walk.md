# src-tauri/src/export/pipeline/plan_walk.rs

Pure bookkeeping for feeding sequential decoders along a `TimeMap::frame_plan`. The screen and webcam pipes only ever move forward one recording frame at a time; the plan says which recording frame each output frame shows; this cursor turns the two into "decode N more frames, then composite the held one". No decoding here, so it is tested to the frame.

## PlanCursor

```rust
pub struct PlanCursor { /* plan, next output index, last decoded recording frame */ }
```

## Step

```rust
pub struct Step { pub j: u64, pub k: u64, pub decodes_needed: u64 }
```

Output frame `j` shows recording frame `k`; `decodes_needed` is how many recording frames the pipes must advance first (the first step counts from frame 0, so a warm-up before a trim-in is `k + 1` decodes; 0 re-uses the held frame, which is slow motion).

## PlanCursor::new

```rust
pub fn new(plan: Vec<u64>) -> Self
```

## PlanCursor::len

```rust
pub fn len(&self) -> usize
```

## PlanCursor::is_empty

```rust
pub fn is_empty(&self) -> bool
```

## PlanCursor::next

```rust
pub fn next(&mut self) -> Option<Step>
```

The next output frame, or `None` after the plan. `frame_loop::run` calls it once per `walk_plan` callback, so the two always describe the same frame.

### Behaviours

- `a_plain_plan_decodes_one_frame_per_output_frame_after_the_warm_up` - `[3, 4, 5]` decodes 4 then 1 then 1.
- `a_cut_or_a_fast_span_skips_frames` - `[0, 2, 4, 40]` decodes 1, 2, 2, 36.
- `slow_motion_reuses_the_held_frame` - `[7, 7, 8, 8]` decodes 8, 0, 1, 0.
- `an_empty_plan_yields_nothing`.
