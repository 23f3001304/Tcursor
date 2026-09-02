# src/editor/timeline/AudioTrack.tsx

One audio track row: the source's waveform image (from `ensureWaveform`) as a quiet, low-contrast background, with a small left icon to tell system vs mic apart. Hidden when that source genuinely wasn't recorded; shows a `Shimmer` skeleton while the waveform fetch is still resolving. Non-interactive - seeks pass through.

## AudioTrack

```tsx
export const AudioTrack: React.MemoExoticComponent<(props: { src: string; kind: "system" | "mic"; loading: boolean }) => JSX.Element | null>
```

`React.memo`'d - `src`/`loading` are set once per project load and never change on a playhead tick or an unrelated edit.

### Props

- `src: string` - the waveform PNG asset URL.
- `kind: "system" | "mic"` - selects the left icon (volume / microphone) and tooltip.
- `loading: boolean` - `Timeline`'s `!wavesReady` (from `useEditorData`). Distinguishes "the waveform fetch hasn't resolved yet" from "it resolved, and this project genuinely has no audio for this source" - `src` is `""` in BOTH cases, so `src` alone can't tell them apart (unlike `Filmstrip`'s `thumbs`, which is unambiguous).

### Behavior

- `!src && loading` - renders `<Shimmer className="e-audiorow" />` (`src/editor/timeline/Shimmer.tsx`), the same box (height/radius/background) as the loaded row.
- `!src && !loading` - renders `null` (this source wasn't recorded; there is nothing to ever show).
- `src` - renders the waveform image as before.
