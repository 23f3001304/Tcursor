# src/hud/components/RecMeter.tsx

The take pill's level slot: one fixed-width box that shows one of two things - the live voice wave, or the word Paused - so the pill never changes width mid-take. `TakeBar` mounts it only while an audio source (mic or system) is on; with both off the slot is absent altogether (owner, 2026-09-14: nothing in the pill for a source that is off). Split out of `Hud.tsx` to keep that file under the line cap; owns no state.

## METER_W

```ts
export const METER_W: number   // 150
```

Width of the meter's drawing area, px, and of the slot. Exported because `useHudWindowSize.ts` needs it: the take window is sized to its content, so the slot's width is a term in `TAKE_WIDTH` (its `SLOT_W`) rather than a number copied into two files that could drift apart.

## METER_H

```ts
export const METER_H: number   // 40
```

Height of the meter's drawing area, px. `VoiceWave` raises its amplitude ceiling to fit (`ceilFor`: 17px either side of the midline at 40), so the wave fills the slot with 3px of air; fits the pill's 60px with room to spare.

## RecMeter

```tsx
export function RecMeter({ live, read, paused }: {
  live: boolean; read: () => { mic: number; sys: number }; paused: boolean;
}): JSX.Element
```

### Props

- `paused: boolean` - the take is paused. Wins over everything else: no level arrives while paused (`Hud.tsx` subscribes `useAudioLevels` only while `recording && !paused`), so a wave here would be a flat line pretending to listen. The slot shows the word Paused instead, breathing on the same 1.6s opacity loop the old chip used.
- `live: boolean` - `useAudioLevels`' `live` flag: true only while level reports are actually arriving from the Rust capture. Distinct from `micOn`, so a permission failure or a device dying mid-take is never rendered as if it were live - `VoiceWave` drains to the line colour (`stale`) while it is false.
- `read: () => { mic: number; sys: number }` - the level getter, passed straight through to `VoiceWave`, which calls it once per animation frame. Not level *values*: at 20 reports a second per source, props would re-render the pill forty times a second.

### Behavior

The two contents swap through `AnimatePresence mode="wait"` keyed on which one is due: the leaving one melts out (opacity 0, scale 0.6, `blur(6px)`, 120ms) and the arriving one springs in from that same frost (scale on a spring at stiffness 420 / damping 15, blur and opacity as short tweens). The swap never changes the slot's width.

### Notes

**State honesty** (task-6 (c)/(i), user-reported: "when muted is selected with no audio and camera the HUD still shows them"). There is no path through this component that can show a level for audio the take is not recording: the levels come from the Rust capture that is writing the WAV, not from a second `getUserMedia` stream in the webview.

**The meter's own geometry is not this file's business.** It hands `VoiceWave` a box and a level getter, and `voiceWave.ts` decides what a frame looks like inside it. That drawing has been through three shapes (a 13-bar strip, two layered sine strokes, the four-layer voice wave) without `METER_W` changing once.

### Used by

- `src/hud/components/TakeBar.tsx` - `{(micOn || sysOn) && <RecMeter live={live} read={read} paused={paused} />}`, between the clock and the Pause button. Nothing mounts it while idle, so `VoiceWave`'s rAF loop only exists during a take.
