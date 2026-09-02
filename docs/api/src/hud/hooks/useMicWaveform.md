# src/hud/hooks/useMicWaveform.ts

Hook that drives a 13-bar microphone level visualizer using the Web Audio AnalyserNode, active only while recording is enabled. Automatically acquires the microphone, runs an `rAF` analysis loop at roughly 20 fps, and releases all audio resources when turned off.

## useMicWaveform

```ts
export function useMicWaveform(on: boolean): { levels: number[]; active: boolean }
```

### Arguments

- `on: boolean` - master on/off switch for the waveform. *Why a boolean rather than a stream:* the hook owns the entire audio pipeline internally so the caller only needs to say "I want levels" or "stop". The caller (`Hud.tsx`) folds `micOn` into this boolean itself, so passing `false` here is the ONLY thing that stops the mic from being opened at all (state honesty, task-6 (c)/(i) - the mic must never open while the user has it toggled off, no matter what `on`'s other inputs are doing).

### Returns

- `levels: number[]` of length 13 (constant `BARS = 13`). Starts as 13 zeros on mount. Resets to zeros whenever `on` becomes `false`.
- `active: boolean` - `true` only once a real `MediaStream` is actually open, `false` before that (mount, still connecting), after a `getUserMedia` failure (permission denied, no device), or after a live track ends (unplug/driver reset/another app taking exclusive access). *Why separate from `on`:* `on` is the caller's INTENT; `active` is the observed truth of whether a device is actually live. The caller renders the wave only when both are true, so a permission failure or a mid-take device loss can never show bars that imply capture is happening.

### Behavior

When `on` becomes `true`:
1. Calls `navigator.mediaDevices?.getUserMedia({ audio: true })` to acquire the default microphone. *Why no device constraint:* the visualizer always uses the system default mic, not the user-selected recording mic.
2. On success: sets `active = true`, attaches `track.onended` to every track (sets `active = false` AND flattens `levels` back to 13 zeros if the mic dies mid-recording, without waiting for a `getUserMedia` failure that will never come - fix round 1, item 3: `active` alone used to flip false while `levels` kept whatever amplitude it last read, so the bars visibly froze instead of going flat), creates an `AudioContext` and an `AnalyserNode` with `fftSize = 64` (32 frequency bins), and pipes the stream through `ctx.createMediaStreamSource(s).connect(analyser)`.
3. Starts a `requestAnimationFrame` loop (`tick`). Each frame increments `frame.current`; frequency data is only read and pushed to state when `frame.current % 3 === 0`, throttling to approximately 20 fps. *Why throttle:* prevents unnecessary React renders at 60 fps for a visualizer that looks fine at 20 fps.
4. Maps the first 13 even-indexed bins (`data[i * 2]`) to `[0, 1]` by dividing by 255. *Why even indices:* samples every other bin to spread the 13 bars across the lower half of the frequency range.
5. On failure (permission denied, no device): sets `active = false`. `levels` stays at 13 zeros.

Cleanup (runs when `on` becomes `false` or the component unmounts):
- Sets a local `cancelled` flag to `true`.
- Cancels the pending `rAF` via `cancelAnimationFrame(raf)`.
- Stops all tracks on the `MediaStream`.
- Closes the `AudioContext`.

When `on` becomes `false`, the effect also resets `levels` to 13 zeros and `active` to `false` immediately (before any async teardown) so the visualizer drops to flat/muted right away.

**Cancellation race (the `cancelled` flag).** `getUserMedia` is async - `on` can flip back to `false`, or the component can unmount, WHILE the permission prompt/device open is still pending. Without the flag, the cleanup above would run with `stream`/`ctx`/`raf` all still at their initial `null`/`0` (nothing to tear down yet), and then the `.then` callback would run anyway once the promise resolved - opening the mic and starting the `rAF` meter loop with nothing left to ever stop it (a permanent mic-open + `rAF` leak). The `.then` callback's first line checks `cancelled` and, if true, immediately stops the just-acquired stream's tracks and returns before creating the `AudioContext` or starting `tick()` - mirroring `useWebcamPreview`'s (`src/hud/hooks/useWebcamPreview.ts`) identical guard.

### Used by

- `src/hud/Hud.tsx` - calls `useMicWaveform(recording && !paused && micOn)` and passes `{ active, levels }` straight into `RecMeter` (`src/hud/components/RecMeter.tsx`).
