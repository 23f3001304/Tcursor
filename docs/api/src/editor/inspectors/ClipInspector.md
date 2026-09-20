# src/editor/inspectors/ClipInspector.tsx

The selected clip's properties: the source range it plays, the stretch of the finished video it
occupies, its place in the export order, the dissolve that runs into it, and Remove. Routed to by
`PropertiesSlot` whenever `sel` matches a `doc.clips` entry - the clip lookup is the LAST rung of
that ladder, so a clip id (`cl0..`) is only ever tried once every other lane has said no
(`PropertiesSlot.md`).

There is no per-clip easing and no per-clip edit set (spec 8): a clip is a span of the recording in
the output order, not a thing with its own look, so the Transition section names the document's
motion curve rather than offering a control for it - `Clip` (`editClips.ts`) carries no `easing`
field, on purpose, and Settings is where that curve is actually changed.

## MAX_TRANSITION_MS

```ts
export const MAX_TRANSITION_MS = 2000;
```

The slider's own ceiling before a neighbour's length narrows it further, mirroring
`clipops::MAX_TRANSITION_MS` in Rust (2000). The two are not imported from one place - TS and Rust
do not share a build - so they are two literals that must agree, the same relationship
`FACTOR_MIN`/`FACTOR_MAX` already has with Rust's own speed clamp.

## transitionCeiling

```ts
export function transitionCeiling(map: TimeMap, i: number): number
```

The slider's actual max for clip `i`: 0 for the first clip (nothing dissolves into it), else
`MAX_TRANSITION_MS` narrowed to half of whichever of clip `i-1` and clip `i`'s own OUTPUT length
(`clipOutMs`, `shared/math/remap.ts`) is shorter. This MIRRORS `clipops::clamp_transition`, which
is the authority - the op re-clamps on every `update_clip` regardless of what the slider allowed -
so the number here exists only so the thumb never has room to drag past where the op would
immediately snap it back, which is what would make the value visibly jump under the pointer.

Takes `map` and the clip's own index rather than the `clips` list: `clipOutMs` reads a clip's
output length off the map's segments (`Segment.clip`), which is all a length comparison needs: the
clip objects themselves never enter the computation.

**The 0 at `i === 0` is the one place this does NOT mirror the op**, and it is safe only because
there is no control there. `clamp_transition` returns `want.min(MAX_TRANSITION_MS)` for index 0, so
Rust will happily STORE up to 2000ms on the first clip; both renderers then ignore it, because the
first clip's span is never the second member of a `clip_spans` pair. The 0 here means "there is no
slider to bound", not "the stored value is 0", and `ClipInspector` renders a `Hint` at that index
rather than a slider clamped to zero. The consequence to know about: a 2000ms transition left on the
first clip becomes a LIVE window the moment that clip is moved later, because `move_clip` carries no
`transition_in_ms` and so never re-clamps it. It can no longer outlive the clip it runs into
(`clipmix.md`, ruling B4-R20), but it can still be longer than this function would have allowed,
until the slider is touched once at the clip's new position.

## ClipInspector

```tsx
export function ClipInspector({ clip, clips, map, dur, motionEasing, onApply, onClose }: {
  clip: Clip; clips: Clip[]; map: TimeMap; dur: number; motionEasing: string;
  onApply: (op: EditOp) => Promise<EditDoc | null>; onClose: () => void;
}): JSX.Element
```

### Props

- `clip: Clip` - the selected clip, found by id in `doc.clips`.
- `clips: Clip[]` - the full list, in output order; `clip`'s own index in it is both its place in
  the export and the index the clock map's segments carry as `Segment.clip`.
- `map: TimeMap` - the clip-to-output clock map (`p.map`, already resolved for the stage and the
  timeline), read for `clipOutMs` and nothing else.
- `dur: number` - the recording's own duration, the source fields' ceiling.
- `motionEasing: string` - `doc.settings.motion.easing`, named on the Transition section rather
  than edited.
- `onApply` - every control applies one `update_clip` or `move_clip` (or `remove_clip` from the
  header); the op re-clamps everything the UI already narrowed.
- `onClose` - drops the selection.

### The sections

| Section | Controls |
|---|---|
| Source | `TimingRow` over `src_in_ms` / `src_out_ms`, the clip's own clock |
| In the export | read only: the output span this clip fills, plus the Order `Picker` |
| Transition | the Dissolve in `Slider`, or a `Hint` saying there is none, for the first clip |

**Source is the one editable range.** `TimingRow` writes `src_in_ms`/`src_out_ms` through
`update_clip`; the op clamps both to the recording's own duration and swaps them back in order if
a drag crosses the other edge, the same normalisation `ClipLane`'s edge drag already relies on.

**The two steppers keep the Clips lane's own floor.** `TimingRow` is handed `minSpanMs={MIN_CLIP_MS}`
(`../hooks/input/useClipDrag.md`, 100ms), so Start rises no further than End minus the floor and End
falls no further than Start plus it. Without it the row was bounded only by the other field, and
`clipops::update_clip` removes a clip whose two bounds MEET - including the last remaining one, which
reverts the whole document to its trim range past `remove_clip`'s own "never the last" guard. The
constant is imported from the lane's hook rather than written again here, so the panel and the pill
can never disagree about how narrow a clip may get. The op is still the authority; this only keeps
the UI from asking for something it would refuse or, worse, honour.

**In the export is derived, and says so by being read only.** The span text is `clipOutMs` summed
over every earlier clip up to this one's own output length - the same arithmetic the Clips lane's
pill label uses - so the inspector and the lane never disagree about where a clip lands after a
cut or a speed span inside it changes its length. The Order `Picker` is the non-drag path to
`move_clip`: picking a number sends `to_index` one less than it (positions are 1-based on screen,
`to_index` is 0-based), the same op the lane's body-drag reorder sends.

**Transition is empty on purpose for the first clip.** `i === 0` renders a `Hint` instead of a
slider - not a disabled slider, not a slider clamped to 0 - because the export and the preview both
ignore `transition_in_ms` on the first clip regardless of what it holds: `ClipMixTrack::resolve`
(`clipmix.rs`) reads a dissolve's length off the INCOMING clip of each adjacent pair in
`spans.windows(2)`, and the first clip's own span is never the second element of any pair, so its
`transition_in_ms` is never read no matter what value it carries. For every other clip, the
`Slider` runs 0 to `transitionCeiling`, step 10ms, formatted `"0.50s"` or `"Hard cut"` at zero,
accented `--e-clip` - the Clips lane's own accent, since a clip is the material rather than an
effect over it.

### Behaviors

`ClipInspector.test.tsx`, jsdom through `inspectorFixture`'s shared root, on the two-clip fixture
`cl1` (6000..9000) then `cl0` (500..4000) over the shared `clipsFixtureMap()`. Each case may patch
the selected clip's own bounds.

- `names the clip by its place in the export and shows both ranges` - "Clip 2", a `0.50s to 4.00s`
  header range, and a hint reading `5.00s to 7.00s` of the finished video.
- `commits a source retime as update_clip` - one Start step sends `{ op: "update_clip", id: "cl0",
  src_in_ms: 600 }`.
- `stops the two steppers a clip floor apart, the same 100ms the Clips lane keeps` - at 3900..4000
  Start's More is disabled, and at 500..600 End's Less is.
- `never sends an update that would collapse the clip to nothing` - at 3850..4000 a Start step would
  land on 3950, 50ms from End, and sends nothing at all. Before the floor it sent that op.
- `caps the transition slider at what the op will allow` - `aria-valuemax` is 1000 on the fixture,
  half of the 2000ms shorter neighbour.
- `says nothing dissolves into the first clip` - no slider at all on `cl1`, and a hint that says so.
- `removes through remove_clip and closes` - the header's delete sends the op and calls `onClose`.

### Notes

- The curve a dissolve runs on is not a field here: `motionEasing` is read and printed in the
  hint, never written, because `Clip` has no `easing` of its own (spec 6.2) - the document's
  Settings panel is where that changes, for every dissolve at once.
- `clips` and `map` must agree on clip order and indices (both come from `doc.clips` through
  `PropertiesSlot`, which is also what `map` was built from), or `clipOutMs`/`transitionCeiling`
  read the wrong segment's length. The two are never independently sourced.
- Remove sends `remove_clip` and closes unconditionally, exactly like every other inspector's
  Delete. The op itself refuses to drop the last clip in the list (`clipops.rs`:
  `if doc.clips.len() > 1`), so Remove on a document's only clip is a no-op the doc absorbs; the
  inspector does not special-case it; it just closes, the same as it would if the op had actually
  removed something.

### Used by

- `src/editor/shell/PropertiesSlot.tsx` - the `clip` branch, reached when `selectedClip` finds
  nothing else matching `sel`.
