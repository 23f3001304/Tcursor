# src/shared/wave/ui/useReducedMotion.ts

## useReducedMotion

```ts
export function useReducedMotion(): boolean
```

Live `prefers-reduced-motion`, checked independent of any Motion `MotionConfig` ancestor.

### Why it is not `MotionConfig`

The editor tree has no `MotionConfig`; only the HUD's does. A component that reads the media query itself honors the setting wherever it mounts, and reacts immediately if the user flips the OS setting without reloading the app.

### Behaviors

- Returns `false` in an environment with no `matchMedia` (so a test renderer never crashes on it).
- Subscribes to the query's `change` event and unsubscribes on unmount.

### Used by

- `src/shared/brand/TcursorMark.tsx` - gates the mark's flow and dot pulse. This hook was lifted out of that file when the wave motif grew past the brand mark; a second copy would have been a second chance to get it wrong.
- `src/shared/wave/ui/VoiceWave.tsx` - selects `voiceFrame`'s phase-frozen variant, and skips the redraw entirely while the level is still.
- `src/shared/wave/ui/SweepWave.tsx` - draws one still frame instead of running the sweep loop.
- `src/shared/wave/ui/QuietWaves.tsx` - stops the drift and the breathing dot.
- `src/editor/timeline/lanes/Playhead.tsx` - drops the drag ripple only, keeping the playhead's own motion.
