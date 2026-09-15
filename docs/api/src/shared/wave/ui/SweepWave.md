# src/shared/wave/ui/SweepWave.tsx

## SweepWave

```tsx
export function SweepWave({ w, h, pct, done, tone }: {
  w?: number; h?: number; pct?: number; done?: boolean; tone?: "ai";
}): JSX.Element
```

The processing wave: one sine sweeping left to right with the dot as its scanning head. Replaces the generic progress bar and the generic spinner at the two moments the app is doing the most work for the user (panel-design benchmark (c) item 7: export and AI progress are a bar or a ring industry-wide).

### Inputs

- `w`, `h` - drawing area in px.
- `pct` - percent complete, or `undefined` for indeterminate work.
- `done` - the completion state: the wave is flat, the flat line fades out, and the dot hands over to a checkmark.
- `tone` - `"ai"` recolours to `--e-ai` for the director's pass; omitted uses `--e-primary`.

### Two modes, one component

With no `pct` (the AI director, which knows steps but not work left) the head is swept from the clock at `SWEEP_PERIOD_S`. With a `pct` the head **is** the percentage: it sits exactly at `pct` of the width on the wave's trailing crest, and `progressAmp` decays the whole wave's amplitude to flat as that reaches 100 - so "nearly done" is legible from the shape before the number is read. There is no second indicator that could disagree with the dot.

### Behaviors

- The waveform itself also travels (one period per `TRAVEL_S`) under the envelope, so the wave reads as flowing rather than as a static shape whose left edge is being revealed.
- `pct` and `amp` are read from a ref inside the draw, never from the effect's dependency list. An export's percent changes many times a second, and if the rAF loop restarted on each one the travel would reset its phase every percent - a visible judder. The loop's only dependencies are `reduced` and `done`; a second, tiny effect covers the still case, redrawing on any prop change while the loop is not running.
- The dot's `cy` is the wave's own y at the head, so the scanning head and the crest are never two separate marks.
- `done` parks the head mid-span rather than off the right edge - the dot has to be somewhere the checkmark can legibly draw itself, and the wave under it is flat by then anyway. The dot scales to zero on a spring, the flat stroke fades to nothing over 0.28s (a `motion.path` opacity tween; the rAF loop keeps writing its `d` through the same ref), and the checkmark draws itself with `pathLength`; these are the stateful transitions here, so Motion owns them while the sweep stays a rAF loop writing path data. The fade exists because a flat full-width accent stroke under a tick read as a red bar with a check drawn over it (owner, 2026-09-14), not as a wave that had settled.
- Under reduced motion the wave is drawn once and the loop never starts: a determinate export still shows real progress (the head is at `pct`), an indeterminate pass shows a still wave, and both Motion transitions collapse to zero duration.

### Styling

`.w-sweep` in `src/shared/wave.css`. `.w-director` (the director's wrapper) is `position: absolute` at `bottom: 102px`, anchored to `.e-stagetoast` exactly the way the Stop pill is: that container is a flex row holding the stage, so anything left in flow there would become a sibling column of the stage rather than an overlay above the pill.

### Used by

- `src/editor/shell/dialogs/ExportProgress.tsx` - determinate, plus the `done` checkmark.
- `src/editor/director/DirectorOverlay.tsx` - `tone="ai"`, indeterminate while planning and determinate at `step/total` once the reveal starts.
