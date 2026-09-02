# src/hud/components/RecMeter.tsx

The recording bar's mic indicator. Split out of `Hud.tsx` to keep that file under the line cap; owns no state - purely a rendering of the truth its props already carry.

## RecMeter

```ts
export function RecMeter({ micOn, active, levels }: { micOn: boolean; active: boolean; levels: number[] }): JSX.Element
```

### Props

- `micOn: boolean` - the HUD's mic toggle. Determines the icon (`Mic`/`MicOff`) and whether a wave or the "Muted" chip renders.
- `active: boolean` - `useMicWaveform`'s `active` flag: `true` only once a real mic stream is actually open. Distinct from `micOn` so a permission failure or a device dying mid-take never gets rendered as if it were live.
- `levels: number[]` - `useMicWaveform`'s `levels`, passed straight to the bars.

### Behavior

- `micOn` false: renders the muted glyph (`MicOff`) and a "Muted" text chip in place of the wave; the container gets the `recmeter muted` class (dimmed via `hud.css`). No stream is ever opened for this state - the caller only calls `useMicWaveform` with `on: true` when `micOn` is also true, so this isn't just a display choice.
- `micOn` true: renders `Mic` and the `.wave` bar row; the row additionally gets the `idle` class when `active` is false (permission pending/denied, no device, or a track that just ended), dimming the bars via CSS so a stalled meter never visually reads as "capturing".

### Notes

State honesty (task-6 (c)/(i), user-reported: "when muted is selected with no audio and camera the HUD still shows them"). No path through this component can render bars for a stream that was never opened.

**Bar geometry (gate-feedback item 2, user-reported 2026-09-02).** Each bar's inline height is `2 + level*18` px - the `2` base matches `hud.css`'s `.wave span { min-height: 2px }` exactly, so silence renders as a short but visible bar, never a bare dot (the earlier `3 + level*18`/`min-height: 3px` pairing was the same idea, just before the redesign shrank the bars). The bars' fixed 3px width/2px gap and the meter's own fixed ~64px total width are CSS-only (`hud.css`'s `.wave`/`.wave span`), not this component's concern - `useMicWaveform.ts`'s `BARS = 13` is what actually sets how many `<span>`s this component maps `levels` into.

### Used by

- `src/hud/Hud.tsx` - renders `<RecMeter micOn={micOn} active={mic.active} levels={mic.levels} />` in the recording bar (in place of the device-selector row shown while idle).
