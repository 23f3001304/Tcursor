# src-tauri/src/export/pipeline/plan_walk.rs

Pure bookkeeping for feeding sequential decoders along a `TimeMap::frame_plan`. The screen and webcam pipes only ever move forward one recording frame at a time; the plan says which recording frame each output frame shows; this cursor turns the two into "decode N more frames, then composite the held one". No decoding here, so it is tested to the frame.

A plan with reordered clips is not monotone, which one forward-only decoder cannot serve, so the file also carries `join_at`: every plan index at which one clip hands over to the next, and whether that handover is one a forward-only decoder can survive. The two halves belong together because the handover that cannot be survived is answered with a respawn, and a respawn is exactly a `rebase`: the new decoder starts at the clip's first source frame, and the cursor has to count its first decode from there.

## PlanCursor

```rust
pub struct PlanCursor { /* plan, next output index, last decoded recording frame, the frame the decoder starts at */ }
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

The warm-up step (the one with nothing decoded yet) counts from `base`, the source frame the decoder was started at, as `k - base + 1`. `base` is 0 until something calls `rebase`, and 0 is today's `k + 1`: a decoder spawned without a seek starts at the file's frame 0, so reaching frame `k` costs `k + 1` decodes. The subtraction saturates, so a `base` past the frame the plan wants still asks for one decode rather than underflowing.

## PlanCursor::rebase

```rust
pub fn rebase(&mut self, base_k: u64)
```

Tells the cursor that the decoders it feeds have just been thrown away and replaced by decoders that start at source frame `base_k`. It sets `base` and clears `decoded`, so the very next step is a warm-up again and counts its decodes from the seek instead of from the file's start: after `rebase(900)` a plan entry of 900 costs one decode, not 901. Nothing else is touched, because the plan and the output index are the same plan and the same index either side of a clip boundary: only the decoder underneath changed.

`frame_loop::run` calls it immediately after `Pipes::rewind` succeeds, with the same `first_k` the respawn seeked to, and never otherwise.

## ClipJoin

```rust
pub struct ClipJoin { pub first_k: u64, pub prev_clip: usize, pub respawn: bool }
```

One clip handing over to the next: the incoming clip's first source frame, the OUTGOING clip's index, and whether the decoders have to be thrown away to serve it.

`prev_clip` is the clip the export is LEAVING, not the one it is entering, because the only thing that wants it is the dissolve's latch: the frame it must blend FROM is the last frame of the outgoing clip.

## join_at

```rust
pub fn join_at(spans: &[ClipSpan], plan: &[u64], j: usize) -> Option<ClipJoin>
```

Whether output frame `j` is the first frame of a clip that is not the first one, and if it is, what kind of join it is. The frame loop calls it once per output frame, before the cursor steps; a `Some` is its cue to latch the outgoing frame, and `respawn` alone decides whether anything more happens.

It looks for the span whose `plan_start` is exactly `j`, so it fires once per span and never in the middle of one. Index 0 is deliberately excluded: `spans[0].plan_start` is always 0 (the map's first segment always lands at least one frame, `remap_spans.md`), so without the guard every export would respawn on its own first frame; the exporter owns that spawn, and it is the no-seek one that keeps a clip-less export identical to what it was. A document with no clips has exactly one span, so this returns `None` for every frame of it.

**The respawn rule (ruling B4-R14): `respawn` is `spans[i].first_k <= plan[j - 1]`, that is, only where the plan goes BACKWARDS.** A forward-only decoder already serves every monotone plan, and has since before clips existed: that is how a cut is skipped, by asking for `k - decoded` decodes and throwing the ones in between away. A forward join is the same thing with a clip boundary on it, so it is decoded through exactly as a cut is, and an in-order split is then byte identical to the unsplit take BY CONSTRUCTION rather than by a seek that has to be argued about. Only a reorder, where the incoming clip's first frame is not after the outgoing clip's last, is something a forward-only decoder cannot do, and only there is a respawn worth its cost.

That cost is real and measured, which is the other half of the reason. The screen decoder's `-ss` is frame exact, but the webcam's is not: a 30 fps camera decoded at `-r 60` has every source frame duplicated across two output ticks, and an accurate seek drops everything before its seek point, so the duplication phase restarts on the first frame at or after it and the panel can run up to one webcam source frame (33 ms at 30 fps) ahead. Respawning at a forward join paid that price for nothing.

### Behaviours

- `a_plain_plan_decodes_one_frame_per_output_frame_after_the_warm_up` - `[3, 4, 5]` decodes 4 then 1 then 1.
- `a_cut_or_a_fast_span_skips_frames` - `[0, 2, 4, 40]` decodes 1, 2, 2, 36.
- `slow_motion_reuses_the_held_frame` - `[7, 7, 8, 8]` decodes 8, 0, 1, 0.
- `an_empty_plan_yields_nothing`.
- `a_cursor_that_is_never_rebased_is_the_shipped_sequence` - all three plans above walk to the same `(j, k, decodes_needed)` triples with `base` in the picture as they did without it.
- `a_rebase_counts_the_first_decode_from_the_seek_not_from_the_file` - `[0, 1, 900, 901]` rebased to 900 at index 2 decodes 1, 1, 1, 1 instead of 1, 1, 899, 1.
- `a_rebase_past_the_frame_it_wants_never_asks_for_a_negative_decode` - `rebase(9)` before a plan entry of 5 asks for one decode.
- `a_reordered_join_respawns_and_names_the_outgoing_clip` - the clips fixture's own map and plan (`remap_clips_tests::clips_fixture`, not a hand-written plan): index 49 yields `first_k 5`, `prev_clip 0`, `respawn true`, because frame 5 is behind the 89 the outgoing clip ended on; 0, 48 and 50 yield nothing.
- `a_forward_join_is_decoded_through_and_never_respawns` - a real `TimeMap::build` of an in-order split at 4000 ms: index 35 yields `first_k 40`, `prev_clip 0`, `respawn FALSE`, because frame 40 is ahead of the 39 the outgoing clip ended on.
- `a_document_with_no_clips_has_one_span_and_no_join_anywhere` - the base fixture's 85-entry plan has one span and no index of it is a join.
