# src/editor/AudioTrack.tsx

One audio track row: the source's waveform image (from `ensureWaveform`) as a quiet, low-contrast background, with a small left icon to tell system vs mic apart. Hidden when that source wasn't recorded. Non-interactive - seeks pass through.

## AudioTrack

```tsx
export function AudioTrack({ src, kind }: { src: string; kind: "system" | "mic" }): JSX.Element | null
```

### Props

- `src: string` - the waveform PNG asset URL; `""` returns `null` (the track hides).
- `kind: "system" | "mic"` - selects the left icon (volume / microphone) and tooltip.
