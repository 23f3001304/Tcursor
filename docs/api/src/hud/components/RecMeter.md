# src/hud/components/RecMeter.tsx

The recording bar's mic indicator: the icon, and the live voice wave beside it. Split out of `Hud.tsx` to keep that file under the line cap; owns no state - purely a rendering of the truth its props already carry.

## METER_W

```ts
export const METER_W: number   // 104
```

Width of the meter's drawing area, px. Exported because `useHudWindowSize.ts` needs it: the recording window is sized to its content, so the meter's width is a term in `RECORDING_WIDTH` rather than a number copied into two files that could drift apart. Unchanged by the voice wave, so the recording pill did not grow.

## METER_H

```ts
export const METER_H: number   // 30
```

Height of the meter's drawing area, px. Tall enough for `level.ts`'s 24px peak-to-peak ceiling, which the mirrored voice wave spends as 12px either side of the midline.

## RecMeter

```tsx
export function RecMeter({ micOn, live, read }: {
  micOn: boolean; live: boolean; read: () => { mic: number; sys: number };
}): JSX.Element
```

### Props

- `micOn: boolean` - the HUD's mic toggle. Determines the icon (`Mic`/`MicOff`) and whether the meter or the "Muted" chip renders.
- `live: boolean` - `useAudioLevels`' `live` flag: true only while level reports are actually arriving from the Rust capture. Distinct from `micOn`, so a permission failure or a device dying mid-take is never rendered as if it were live.
- `read: () => { mic: number; sys: number }` - the level getter, passed straight through to `VoiceWave`, which calls it once per animation frame. Not level *values*: at 20 reports a second per source, props would re-render the recording bar forty times a second.

### Behavior

- `micOn` false: renders the muted glyph (`MicOff`) and a "Muted" text chip in place of the meter; the container gets the `recmeter muted` class (dimmed via `hud.css`). No stream is ever opened for this state - `Hud.tsx` only subscribes `useAudioLevels` while a take is genuinely recording an enabled source, so this is not just a display choice.
- `micOn` true: renders `Mic` and the `VoiceWave`, which drains its colour (`stale`) when `live` is false.

### Notes

**State honesty** (task-6 (c)/(i), user-reported: "when muted is selected with no audio and camera the HUD still shows them"). There is no path through this component that can show a level for audio the take is not recording: the levels come from the Rust capture that is writing the WAV, not from a second `getUserMedia` stream in the webview.

**The meter's own geometry is not this file's business.** It hands `VoiceWave` a box and a level getter, and `voiceWave.ts` decides what a frame looks like inside it. That drawing has been through three shapes now - a 13-bar strip, then two layered sine strokes with the brand dot as a peak follower, and now the four-layer voice wave - without this file's props or `METER_W` changing once, which is the point of the split.

### Used by

- `src/hud/Hud.tsx` - renders `<RecMeter micOn={micOn} live={audio.live} read={audio.read} />` in the recording bar, in place of the device-selector row shown while idle. Nothing mounts it while idle, so `VoiceWave`'s rAF loop only exists during a take.
