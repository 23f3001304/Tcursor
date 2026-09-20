# src/editor/stage/clips/clipDissolve.ts

The preview's half of the clip dissolve (Batch 4 T7), and a line-for-line mirror of `src-tauri/src/export/render/clipmix.rs`, which is the reference. At a clip boundary the export cross-dissolves the PICTURE without overlapping the clock: it holds the outgoing clip's last frame and blends it into the incoming clip's first `transition_in_ms`, on the SCREEN plane only, so the webcam and the background are untouched and the exported duration is the duration the document had with the transition at zero. This file answers the three questions the canvas needs to draw that same thing live: where the windows are, what the mix is at one instant, and which source instant the second `<video>` should be holding.

The three functions are pure and take the map and the clip list as arguments, so the engine can recompute the list from a memo and the rAF tick can ask for one instant without touching React. Nothing here draws: `drawScreenPanel` (`../canvas/previewDraw.md`) paints the two frames, `useStageEngine` (`../useStageEngine.md`) owns the memo and the element, and `compositeFrame` (`../../hooks/stage/compositeFrame.md`) is where the two meet.

## PREVIEW_FPS

```ts
export const PREVIEW_FPS = 60
```

The frame rate `clipDissolves` plans against, and it must equal Rust's `OUT_FPS` (`src-tauri/src/export/render/mod.rs`, 60). A dissolve window opens on the PLAN index the incoming clip starts at, not on its segment's `outStart`: `outStart` accumulates fractional segment lengths while the plan counts whole frames, and on the shared clips fixture at 10 fps those two are 4900 and 5000, a whole frame apart. The plan index depends on the fps it was counted at, so a preview counting at a different rate would open its window somewhere else and the canvas would stop agreeing with the file.

**The two previews plan at 60; the EXPORT plans at whatever the user picked.** `ClipMixTrack::resolve` is handed the export's own fps, so a 30 fps export opens the same window on its own plan index and can land up to one 30 fps frame, 33 ms, from where these two previews draw it. Matching `OUT_FPS` is what keeps the paused frame and the live canvas on the same instant as each other and as a 60 fps export, which is the default; a non-default export rate is a known and accepted skew rather than a defect on either side.

## PRESEEK_LEAD_MS

```ts
export const PRESEEK_LEAD_MS = 1000
```

How far ahead of a window the second `<video>` is asked for the outgoing tail. A `currentTime` assignment resolves in roughly 30 to 120 ms in WebView2 (spec 6.5, and the reason Task 6's clock is element-slaved), so asking at the boundary would open the dissolve on whatever stale frame the element still held. A second is generous on purpose: the element is paused and hidden, the seek costs one decode, and the only thing the lead buys is that the frame is already there.

## ClipDissolve

```ts
export interface ClipDissolve { outStartMs: number; prevClip: number; durMs: number }
```

One window, in output milliseconds: where it opens, which clip is dissolving OUT of it (an index into the document's clip list), and how long it runs. The same three fields as Rust's `ClipDissolve`, with the same meanings, so a row of one test table is a row of the other.

## ClipMix

```ts
export interface ClipMix { prevClip: number; prevOutMs: number; alpha: number }
```

The mix at one output instant. `alpha` is the INCOMING clip's weight, so the panel is `alpha * incoming + (1 - alpha) * outgoing` and the boundary frame, where `alpha` is 0, is entirely the outgoing clip. `prevOutMs` is the last output millisecond before the window; the export latches on the PLAN entry that millisecond falls in, which is the lookup `preseekAt` makes rather than a mapping back to source.

## clipDissolves

```ts
export function clipDissolves(map: TimeMap, clips: Clip[], fps = PREVIEW_FPS): ClipDissolve[]
```

Every window this document has, walked off `clipSpans` (`../../../shared/math/remapPlan.md`, Task 1) exactly as `ClipMixTrack::resolve` walks `map.clip_spans`: one window per adjacent pair of spans, opening at `floor(planStart * 1000 / fps)` of the SECOND span, carrying the first span's clip as `prevClip` and the second clip's stored `transition_in_ms` as its duration. A zero (or missing) transition contributes no window, which is why the first clip's stored transition is ignored: it is never the second member of a pair, so nothing dissolves into it. A document with no clips has one span and therefore no pairs, and the empty list it returns is what makes every downstream branch in this task disappear.

**The duration is clamped to the incoming span, `Math.min(want, Math.floor(planLen * 1000 / fps))`** - the mirror of Rust's `dur_ms.min(plan_len * 1000 / fps)`, ruling B4-R20. A window that outlived its own clip was reachable through ordinary editing (a clip edge dragged short leaves its old dissolve over-long, and `clamp_transition` exempts clip 0, so a dissolve parked there survives a reorder), and this preview had none of the export's protection: the export drops the blend at the next join because its latch no longer matches `prevClip`, while `clipMixAt` kept answering the FIRST window in the list and `drawScreenPanel` kept painting the earlier clip's tail over the clip after next at a rising alpha. Clamping where the window is built ends both the ghost and the masking, and keeps this function the same arithmetic as `resolve`. A clamp that lands on zero pushes no window, exactly as a stored zero does not.

## clipMixAt

```ts
export function clipMixAt(list: ClipDissolve[], outMs: number, easing: string): ClipMix | null
```

The mix at `outMs`, or `null` outside every window. The window is half open, `[outStartMs, outStartMs + durMs)`, so the first frame of the incoming clip after the transition is undissolved. `alpha` is `ease(easing, (outMs - outStartMs) / durMs)` on the document's own motion easing through `ease` (`../../timeline/model/layoutTrack.md`), the same curve the layout transitions run on.

**The two evaluators agree** (ruling B4-R1). Task 3 built the export's dissolve on `crate::export::camera::ease`, whose `Smooth` is `p * p * (3 - 2p)`, which is exactly what this `ease` computes for an unrecognised name such as `"smooth"`. So the alpha table is pinned on BOTH sides for linear AND smooth: 0.5 at 5150 and 0.15625 at 5025 are the numbers `clipmix_tests.rs` asserts and the numbers `clipDissolve.test.ts` asserts, with no per-side allowance between them.

`outMs` is the loop's own `tOut`, the output clock Task 6 gave it. At a non-contiguous boundary that clock holds at the incoming segment's `outStart` while the element seeks, and a held `tOut` near the window's start answers an alpha near 0, so the picture on those ticks is the pre-seeked outgoing tail: the frame the loop holds while the seek lands and the dissolve's first frame are the same picture, which is what the export renders too.

## preseekAt

```ts
export function preseekAt(list: ClipDissolve[], map: TimeMap, outMs: number, fps = PREVIEW_FPS, leadMs = PRESEEK_LEAD_MS): number | null
```

The SOURCE instant the second `<video>` should be parked on at `outMs`, or `null` when no boundary is near. The search window is the dissolve widened by `leadMs` at its front, `[outStartMs - leadMs, outStartMs + durMs)`, so the element is asked for the frame a second before it is needed and keeps being asked for it through the whole dissolve (an idempotent assignment: the pre-seek effect only writes `currentTime` when it is more than 50 ms off). `fps` is the rate the dissolve list was built at, because the answer is a plan lookup and the plan is counted at one rate.

**It is `latched_ms` written in TypeScript** (ruling B4-R16, now applied to all three renderers). The instant is `framePlan(map, fps)[floor(prevOutMs * fps / 1000)]`, the plan entry the export's walk was standing on when it latched the outgoing picture, and `null` when the plan has no such entry. It is deliberately NOT `clipOf(map, prevOutMs)`, the continuous inverse, which differs by a whole source frame at a boundary: on the clips fixture at 60 fps the window opens at plan index 299, so `prevOutMs` is 4982; `clipOf` answers 8982 ms, inside source frame 538, while `plan[298]` is frame 539, which is the frame the export blended and the frame `compose.rs` decodes for the paused preview. One frame, on a held still, reads as a ghost mis-registration on any moving shot.

**Why the MIDDLE of that frame, `(k * 1000 + 500) / fps`.** A `<video>` resolves a `currentTime` assignment to the frame whose interval CONTAINS it, at or before, so an instant sitting on frame `k`'s own leading edge is the one place a rounding error lands on `k - 1`. Half a frame in is as far from both edges as an instant can get: frame 539 at 60 fps is `[8983.33, 9000)` and the assignment is 8991.67.

### Behaviors

`clipDissolve.test.ts`, on the shared `clipsFixtureMap()` whose six segments are pinned in `remapClips.test.ts` and whose two clip spans at 10 fps are `{clip 0, planStart 0, planLen 49}` and `{clip 1, planStart 49, planLen 21}`. The clip list is the fixture's own pair with the two transitions as parameters, which is the shape `clipmix_tests.rs` uses.

- `opens the window on the plan boundary, not on the segment's outStart` - a 500 ms transition on the second clip resolves to `{outStartMs: 4900, prevClip: 0, durMs: 500}`, the plan index 49 and not the 5000 ms `outStart`.
- `ignores the first clip's stored transition` - 400 ms on the first clip and nothing on the second resolves no window at all.
- `has nothing to dissolve on a document with no clips` - the plain fixture, one span, no pairs.
- `clamps a transition longer than its own clip to the incoming span, the row Rust also pins` - 5000 ms on the second clip resolves to `durMs` 2100, the span's 21 plan entries at 10 fps, and 500 ms is stored as it is. The same two numbers `a_transition_longer_than_its_own_clip_is_clamped_to_the_incoming_span` asserts in Rust.
- `is null outside the half open window` - 4899 and 5400 are both outside it.
- `carries the incoming clip's weight on a linear curve, the one row Rust also pins` - 4900 is `{prevClip: 0, prevOutMs: 4899, alpha: 0}`, 5150 is 0.5 and 5399 is 0.998.
- `carries the smooth row Rust pins too, the same curve on both sides of the boundary` - 5150 is 0.5 and 5025 is 0.15625, which is `0.25 * 0.25 * (3 - 0.5)` where linear would give 0.25.
- `lets the second join answer for itself once the first window cannot outlive its clip` - a three-clip map of 3000 ms pieces with 5000 ms on clip 1 and 400 ms on clip 2 resolves `[3000, 6000)` from clip 0 and `[6000, 6400)` from clip 1, and 6100 reads `{prevClip: 1, prevOutMs: 5999, alpha: 0.25}`. Unclamped, the first window still held 6100 and answered clip 0. Rust pins the same two windows and the same mix in `an_over_long_window_no_longer_masks_the_next_joins_own_dissolve`.
- `asks for the middle of the latched frame a second ahead of the boundary and through the window` - 3900, 4899 and 5399 all answer 8950: the window opens at plan index 49, so `prevOutMs` is 4899, `jPrev` is 48, `plan[48]` is source frame 89, and the middle of frame 89 at 10 fps is 8950 ms.
- `holds the frame the export latched, not the source frame before it, at the preview's own 60 fps` - the same fixture planned at 60 opens its window at 4983, and the answer is 8991.67 ms, source frame 539, where `clipOf(map, 4982)` answers 8982 ms and source frame 538.
- `asks for nothing when no boundary is near` - 3899 is one millisecond before the lead opens, 1000 is nowhere near, 5400 is one past the window's end.
- `asks for nothing at all when no clip dissolves` - an empty list answers `null` wherever it is asked.

### Used by

- `../useStageEngine.md` - memoises `clipDissolves(p.map, p.clips)` and `preseekAt(dissolves, p.map, tOut)` once per render, returns `wantsScreenB: dissolves.length > 0` for the second element, and mirrors the list and the easing into the loop through `useSyncRefs`.
- `../../hooks/stage/compositeFrame.md` - calls `clipMixAt` once per painted tick and hands the result to `drawPreview` as its `mix`.
