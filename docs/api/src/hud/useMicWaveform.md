# src/hud/useMicWaveform.ts

Hook that drives a 14-bar microphone level visualizer using the Web Audio AnalyserNode, active only while recording is enabled. Automatically acquires the microphone, runs an `rAF` analysis loop at roughly 20 fps, and releases all audio resources when turned off.

## useMicWaveform

```ts
export function useMicWaveform(on: boolean): number[]
```

Returns a `number[]` of length 14 where each element is a normalized frequency magnitude in `[0, 1]`, sampled from the default audio input.

### Arguments

- `on: boolean` - master on/off switch for the waveform. *Why a boolean rather than a stream:* the hook owns the entire audio pipeline internally so the caller only needs to say "I want levels" or "stop"; it does not need to manage a `MediaStream` separately.

### Returns

`number[]` of length 14 (constant `BARS = 14`). Starts as 14 zeros on mount. Resets to zeros whenever `on` becomes `false`.

### Behavior

When `on` becomes `true`:
1. Calls `navigator.mediaDevices?.getUserMedia({ audio: true })` to acquire the default microphone. *Why no device constraint:* the visualizer always uses the system default mic, not the user-selected recording mic.
2. Creates an `AudioContext` and an `AnalyserNode` with `fftSize = 64` (yielding 32 frequency bins).
3. Pipes the stream through `ctx.createMediaStreamSource(s).connect(analyser)`.
4. Starts a `requestAnimationFrame` loop (`tick`). Each frame increments `frame.current`; frequency data is only read and pushed to state when `frame.current % 3 === 0`, throttling to approximately 20 fps. *Why throttle:* prevents unnecessary React renders at 60 fps for a visualizer that looks fine at 20 fps.
5. Maps the first 14 even-indexed bins (`data[i * 2]`) to `[0, 1]` by dividing by 255. *Why even indices:* samples every other bin to spread the 14 bars across the lower half of the frequency range.

Cleanup (runs when `on` becomes `false` or the component unmounts):
- Cancels the pending `rAF` via `cancelAnimationFrame(raf)`.
- Stops all tracks on the `MediaStream`.
- Closes the `AudioContext`.

When `on` becomes `false`, the effect also resets state to 14 zeros immediately (before any async teardown) so the visualizer bars drop to flat.

Errors from `getUserMedia` are silently caught; the state remains at 14 zeros.
