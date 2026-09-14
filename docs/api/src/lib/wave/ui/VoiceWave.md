# src/lib/wave/ui/VoiceWave.tsx

## VoiceWave

```tsx
export function VoiceWave({ w, h, read, live }: {
  w?: number; h?: number; read: () => { mic: number; sys: number }; live: boolean;
}): JSX.Element
```

The recorder's live level meter: a centre line with a dot at each end, and four overlapping translucent lenses across the middle that swell with the input. Replaces `WaveMeter.tsx` (two open sine strokes and a needle dot), which the owner rejected on sight.

Each lens is a closed mirrored sine (`lensPath`) filled from one blue-to-violet ramp shared by all four. The layers differ only in phase, wavelength, amplitude and drift direction, so they sweep through each other; where two overlap the colour deepens because two translucent fills composited, not because anything painted a third colour there.

### Inputs

- `w`, `h` - the drawing area in px. `RecMeter.tsx` owns the numbers the HUD uses (`METER_W` 150 / `METER_H` 40), which `useHudWindowSize.ts` reads to size the take window. The amplitude ceiling follows `h` (`voiceWave.ts`'s `ceilFor`: `h / 2 - 3`, so 17px either side of the midline at 40, and the 30px design's 12 there), so the wave fills whatever slot it is given with 3px of air.
- `read` - called once per frame for the newest RMS pair. A getter, not props: at 20 reports a second per source, level props would re-render the whole recording bar forty times a second to move a wave this component's own rAF loop is already redrawing. Must be referentially stable (`useAudioLevels` returns a `useCallback`'d one) or the loop re-subscribes.
- `live` - the honesty gate. `false` (a source is enabled but no level has arrived: permission pending, no device, a driver reset mid-take) adds the `stale` class, draining the colour out of the layers rather than letting a resting blue-to-violet wave imply capture that is not happening.

### Behaviors

- **No `setState` anywhere.** The rAF loop calls `voiceFrame`, then writes one `d` per layer. React renders this component only when its props change.
- **The loop lives and dies with the mount**, and `RecMeter` only mounts while a take is running, so nothing is drawn when the HUD is not recording.
- **The gradient is `gradientUnits="userSpaceOnUse"`**, spanning `0..w` of the meter rather than each path's own bounding box. With the default object-bounding-box units a short layer would show the whole blue-to-violet span inside its own width and the layers' colours would not line up where they cross.
- The generated `id` for that gradient is `useId()` with its punctuation stripped, because React's ids carry characters that a `url(#...)` reference cannot.
- **Reduced motion is not a second render path.** `voiceFrame` returns a phase-frozen state and the same writes apply. The loop additionally skips the write entirely while the level has not moved by `STILL_EPSILON_PX`, so a still meter costs nothing during a recording.
- The centre line and the two end dots are drawn *under* the layers, so a loud take washes over them instead of being cut by them. The taper guarantees the wave meets the line at both ends, so the three read as one object rather than a wave parked on a rule.

### Styling

`src/lib/wave.css`, imported by this file. `.w-voice` binds the gradient stops to `--wave-a` / `--wave-b` (defined per theme in `src/hud/hud.css`: `#3b82f6`/`#8b5cf6` light, lifted to `#60a5fa`/`#a78bfa` dark, since the layers are 40-58% opaque and the light pair loses its luminance against the dark bar). `.w-voice.stale` drops the layers to the line colour. There is deliberately **no** `drop-shadow` glow here, unlike the stroked waves: at four overlapping layers a glow only muddies the crossings the shape is built to show.

### Used by

- `src/hud/components/RecMeter.tsx` - the take pill's level slot.
