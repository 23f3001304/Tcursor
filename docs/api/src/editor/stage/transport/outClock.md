# src/editor/stage/transport/outClock.ts

The preview's playback rule for a document whose clips were split or reordered (Batch 4 T6). `playback.ts` beside it is the single-clip rule and stays the contract for every document with at most one clip contributing frames; this file is the one that can traverse a clip list whose OUTPUT order walks the source backwards, where the screen `<video>` left to itself just plays on into whatever happens to follow in the file.

The clock says WHICH segment is playing, the element says WHERE inside it. That division is the whole design: `TimeMap.segments` are in output order and each carries its own source range, so at a segment's end the loop knows what comes next, while inside one the element's `currentTime` is the only honest measure of where the decoder actually is.

**Why the clock is not a wall clock.** The first design (spec 6.3) advanced output time by `performance.now()` deltas and seeked the media whenever the element was more than one frame from where that clock said it should be. A `currentTime` assignment takes 30 to 120 ms in WebView2 and, while it lands, `currentTime` reads back the seek TARGET: the wall clock runs on while the picture stands still, the drift rule fires again two ticks later, the new seek aborts the one still landing, and a multi-clip take plays as a frozen frame over silent audio for as long as it runs (the composite is skipped on a seek tick, which is what a cut has always done). Element-slaved, that cannot happen, because a tick that finds the element exactly where a pending seek asked for it returns no seek at all, however many times it is asked (behaviour `does not re-seek while a slow seek is still landing`). Neither this file nor the loop reads `performance.now()`.

## ClipTick

```ts
export interface ClipTick { tOut: number; t: number; seekTo: number | null; ended: boolean }
```

One tick's answer: the new value of the output clock, the clip instant to draw, a source instant to seek all three media elements to (or `null`), and whether the output is over. `tOut` is NOT rounded here: the loop keeps the exact number as its clock state and rounds only what it hands the painter, which is `outOf`'s integer contract and what Task 5's `exactKey` compares.

## needsOutClock

```ts
export function needsOutClock(map: TimeMap): boolean
```

Whether this document needs the clock at all: true only once a SECOND clip contributes segments. The test is on the segment list rather than on `doc.clips.length > 1` because `buildTimeMap` drops a clip whose source range is inverted or empty, and a dropped clip must not flip the loop into a mode it does not need. With one clip left, however it was trimmed, the shipped element-driven loop is exactly right, and it stays byte for byte itself: `useCompositeLoop` computes no tick and takes no new branch while this is `false`.

## clipTick

```ts
export function clipTick(map: TimeMap, outMs: number, mediaMs: number, frameMs = FRAME_MS): ClipTick
```

`outMs` is the clock's value from the previous tick, `mediaMs` is `screen.currentTime * 1000` right now. The clock is clamped to `[0, outDurMs(map)]`, the segment whose OUTPUT interval `[outStart, outStart + (clipEnd - clipStart) / factor)` holds it is found (the last segment when the clock is at the very end), and one of four branches answers:

1. **On track**, `from.clipStart - frameMs <= mediaMs < from.clipEnd`. `t = max(mediaMs, from.clipStart)`, `tOut = from.outStart + (t - from.clipStart) / from.factor`, no seek. The clock FOLLOWS the element, jumps included: an element further into the segment than the clock simply moves the clock to it, at the segment's own speed (source 7000 in the 0.5x segment is output 2000, not 1000). The frame of slack before the start is for a seek that landed a hair early, since `currentTime` is honoured to the nearest decodable frame and not exactly.
2. **Ran off the end**, `mediaMs >= from.clipEnd` by no more than `NATURAL_TICK_MAX_DELTA_MS` (500, `playback.ts`'s own "was this an ordinary playback tick" window; anything further is a stray element, case 4). Step to the next segment IN OUTPUT ORDER, and keep stepping while that one is contiguous in the source (`next.clipStart === cur.clipEnd`: a speed edge, or a split left in order) and the element is past its end too. That loop is what walks over a segment shorter than one tick instead of seeking backwards into it.
3. **The end of the output**, when the step finds no next segment. `tOut` is the output's end, `t` is the last whole millisecond the map shows (`max(last.clipStart, last.clipEnd - 1)`), there is no seek and `ended` is true.
4. **Anywhere else**: a fresh Play, or an element something else moved. `t = clipOf(map, clock)`, the clock keeps its own value, and the element is seeked to `t` so the next tick is case 1.

A contiguous next segment in case 2 needs NO seek: the element is already playing the right source, so the branch falls through to case 1's arithmetic on that segment and only `factor` and `outStart` change under it. A NON-contiguous one (a cut, a clip join, a reorder) is the one place this file seeks: `tOut = next.outStart`, `t = next.clipStart`, `seekTo = next.clipStart`.

`ended` is a report and not a state: it is true on every tick the element is past the last segment, and the loop's answer is to call `onSeek(t)` (`useEditorCallbacks.ts`: `setPlaying(false); setTimeMs(ms)`), which stops playback and parks the playhead on the last frame that was shown. Stopping has to be the loop's job here, because the editor's own end rule, `onTime`'s `ms >= trim.outMs`, is on SOURCE time: the last clip of a reordered take ends in the middle of the recording, so that comparison never fires and the audio would play on into material no clip shows. Parking at `t` rather than at the trim-out keeps the playhead, the quick preview and the Rust exact frame on one picture.

### Known limit

After a reordered take has ended, Play at the parked playhead runs for a millisecond and stops again. `onPlayToggle`'s restart rule compares the playhead against the SOURCE trim range and the parked instant is inside it, so nothing rewinds; moving the playhead first plays as expected. That rule belongs to the editor's transport rather than to the loop and was deliberately left alone in this task.

### Behaviors

`outClock.test.ts`, mostly on the shared `clipsFixtureMap()` whose six segments are pinned in `remapClips.test.ts` as `[clip, clipStart, clipEnd, factor, outStart]` = `[0,6000,8000,0.5,0] [0,8000,9000,1,4000] [1,500,1000,1,5000] [1,2000,2500,1,5500] [1,2500,3500,2,6000] [1,3500,4000,1,6500]`, output duration 7000.

- `is false for a document nobody has split`
- `is false for a single clip, however it was trimmed`
- `is true as soon as a second clip contributes frames`
- `is false when every clip but one is empty` - an inverted range never reaches the segment list, so the loop stays in its shipped mode.
- `follows the element inside the segment the clock is in` - `(0, 7000)` answers `tOut` 2000, and `(5250, 766)` answers 5266.
- `gives an element that landed a hair early one frame of slack` - at clock 5000, media 490 draws source 500 with no seek, media 480 seeks to 500.
- `crosses a contiguous join without a seek` - `(3990, 8004)` is `tOut` 4004 on the segment that starts at source 8000, and `(5990, 2510)` is 6005 inside the 2x span.
- `seeks at a non contiguous join, to the next segment in OUTPUT order` - `(4995, 9003)` leaves source 9000 for source 500, the reorder; `(5495, 1002)` takes the cut inside the second clip.
- `does not re-seek while a slow seek is still landing` - `(5000, 500)` five times in a row, feeding each answer's `tOut` back in, returns the same answer every time.
- `ends the output when the element runs off the last segment` - `(6990, 4001)` is `tOut` 7000, `t` 3999, `ended`; `(6990, 3995)` is still playing at 6995.
- `puts a stray element where the clock is` - `(5250, 8900)` seeks to 750, `(0, 750)` seeks to 6000.
- `clamps the clock to the output at both ends` - a negative clock is 0, and one past the end is 7000 and `ended`.
- `steps over a contiguous sliver the element has already passed` - a 10 ms 2x span between two clips: `(995, 1015)` lands on the segment after it at `tOut` 1010, with no seek.
- `is already over when the clip list shows nothing` - a map with no segments answers `trimIn` and `ended`, so the function is total.

### Used by

`useCompositeLoop` (`../../hooks/stage/useCompositeLoop.md`) and nothing else. Task 7's second hidden video draws its dissolve from the same `tOut`, so this shape is a contract rather than a private helper.
