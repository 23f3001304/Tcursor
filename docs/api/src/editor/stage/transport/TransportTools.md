# src/editor/stage/transport/TransportTools.tsx

The transport's left tool group, moved out of `Transport.tsx` (at the size cap) when the time remap gave the transport a second clock to show, and grown in the same milestone with the two actions that create a cut and a speed span.

## CLICK_LOOKAHEAD_MS

```ts
export const CLICK_LOOKAHEAD_MS = 8000;
export const DEFAULT_SPAN_MS = 4000;
```

How far past the playhead a recorded click still counts as "the end of what I am doing now", and the block Cut and Speed fall back to when there is no click that soon. Both from the spec (section 6).

## DEFAULT_SPAN_MS

See `CLICK_LOOKAHEAD_MS` above - the two are one decision and are declared together.

## actionSpan

```ts
export function actionSpan(range: Range | null, nowMs: number, clicks: ClickSample[], dur: number): Range
```

The clip-ms span Cut and Speed act on:

1. the ruler's selection when there is one (`timeline/useRangeSelect.ts`), whatever the playhead and the clicks say;
2. otherwise, from the playhead to the EARLIEST recorded click that is both ahead of it and within `CLICK_LOOKAHEAD_MS` - the clicks are not assumed sorted, so this is a reduce, not a `find`;
3. otherwise, a `DEFAULT_SPAN_MS` block from the playhead.

The end is clamped into the clip; the start is clamped at 0. Pure, so the fallback that actually fires on a first click of the button is pinned by a test rather than by clicking it.

## TransportTools

```tsx
export function TransportTools({ locked, trimmed, onTrimIn, onTrimOut, onResetTrim, onAddZoom, onAddText, onAutoedit, aiRunning, exporting, timeMs, dur, clicks, range, setRange, onApply, onDetectSilences }): JSX.Element
```

Trim start/end to the playhead (the timeline edge handles do the same), a reset that appears once trimmed (Motion press spring), then a divider and the timeline tools: **Cut** (`IconCut`), **Speed 2x** (`IconPlayerTrackNext`), add a zoom, add a text item (`IconTypography`, `onAddText`, titled with its `(T)` shortcut and placed directly beside the zoom tool because the two are the same gesture - drop an element at the playhead - and Z and T are neighbours in the keymap), run the AI director. The wand is disabled while a run or an export is in progress so a double click cannot fire two interleaved reveals, and carries `data-director-anchor="wand"` for the director's fake pointer. `locked` is the transport's own `exporting || dur <= 0` and disables Cut and Speed too - neither is meaningful with no clip, and neither may change the doc mid-export.

### Magnetic pull (micro-interaction pass, 2026-09-14)

Trim In and Trim Out lean toward a pointer that comes within 28px of them, through [useMagnetic](../effects/useMagnetic.md). They are the only tools in this group that are a *decision* rather than a toggle, and they are the pair Play is flanked by - so the three of them lean together as the pointer crosses the bar, which is the whole reason the effect is worth having here and nowhere else in the group.

The `x`/`y` MotionValues go **straight onto the buttons**: every press in the transport is scale-only, so nothing is claiming the translate channel and no wrapper element is needed anywhere (see `useMagnetic.md`, "The `x`/`y` channel must be free"). `motion.button` plus a `style` is all these two gain - the press spring the look pass took off them stays off.

`locked` passes `strength: 0`, which makes the hook a full no-op - no listener, no spring - so a pill that will ignore the click does not lean toward the pointer inviting one. That is the same rule the press spring already follows one paragraph up.

The two hooks are one `pointermove` listener each on `window`, rAF-coalesced, and both disappear entirely when `interface_effects` is off or the OS asks for reduced motion.

### Cut and Speed

Both run the same two lines: take `actionSpan(range, timeMs, clicks, dur)`, apply one op over it when it is non-empty (`add_cut`, or `set_speed` at factor 2), then clear the range. Clearing is what makes the gesture read as "choose, then act" rather than leaving a stale selection that the next press would silently reuse.

Each carries `data-action="cut"` / `data-action="speed"` (the way the wand carries its own anchor attribute) so the spec can press the real button rather than calling a handler. The tooltips name the gesture - "Remove the selected range from the clip. Shift+drag the ruler to choose a range." - because the range gesture has no other affordance on screen; `ShortcutsOverlay` lists it as well.

**Remove silences** (`IconEarOff`, `data-action="silences"`) sits between Speed and the zoom tool, next to the two manual ways of making a cut. It only calls `onDetectSilences` (`hooks/doc/useSilences.ts`): the scan, the one `add_cuts` and the toast live there, so the button touches neither the doc nor the range itself; disabled with `locked` like Cut and Speed.

### Props

- `timeMs: number` - clip time, where a range-less Cut or Speed starts. A plain prop rather than a ref: `Transport` re-renders every tick for its readout anyway, so there is no memo to protect here.
- `dur: number` - the clip's duration, the span's ceiling.
- `clicks: ClickSample[]` - the recording's clicks on clip time (`SlotProps.clicks`, the same array the preview draws), the lookahead's input.
- `range: Range | null` / `setRange` - the ruler's selection and the way to clear it.
- `onApply` - the op sink (`Editor.tsx`'s `applyOp`), so Cut and Speed are one undo step each like every other edit.
- `onDetectSilences` - Remove silences' callback, from `SlotProps`.
