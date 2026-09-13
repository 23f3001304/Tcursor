# src/hud/hooks/useAudioLevels.ts

## AudioLevel

```ts
interface AudioLevel { source: "mic" | "system"; rms: number }
```

Payload of the Rust `audio-level` event. Mirrors `AudioLevel` in `src-tauri/src/audio/level.rs`.

## STALE_MS

```ts
const STALE_MS: number   // 400
```

A source is treated as live only while its reports keep arriving. The Rust threads report every ~50ms (`LEVEL_POLL_MS`), so three missed windows plus slack is decisive without being twitchy.

## useAudioLevels

```ts
export function useAudioLevels(on: boolean): { read: () => { mic: number; sys: number }; live: boolean }
```

Live 0..1 RMS for the recorder's two audio sources, straight from the Rust capture that is writing the WAV.

### Why not `getUserMedia`

The hook it replaced (`useMicWaveform`) opened a **second** microphone stream in the webview while the Rust recorder already had the device open, and measured that copy rather than the take. This one measures the capture itself, which means the meter cannot show a level for audio the take is not recording, the app never opens the microphone twice, and system audio - which the webview had no way to see at all - gets a real level for the meter's back stroke.

### Returns

- `read` - a getter, not state. At 20 reports a second per source, storing these in React state would re-render the whole recording bar forty times a second to move a wave `VoiceWave`'s own rAF loop is already redrawing. Referentially stable (`useCallback` with no deps), so a memoised meter never re-subscribes. Each source reads `0` once its last report is older than `STALE_MS`, so a dead input goes quiet rather than freezing at its last live-looking value.
- `live` - real state, because it changes a handful of times per take (a device opening, a driver resetting, a mic toggled off) and gates the meter's honest greyed-out look.

### Behaviors

- Subscribes only while `on`; unsubscribes and clears the cached levels when it flips false, so a paused take reports nothing - which is exactly what the meter should show.
- A 250ms interval drops `live` when reports stop. A take can lose its input mid-recording (device unplugged, driver reset, another app taking exclusive access) and the reports simply stop, so nothing else would notice.

### Used by

- `src/hud/Hud.tsx` - subscribed while `recording && !paused && (micOn || sysOn)`, handed to `RecMeter`.
