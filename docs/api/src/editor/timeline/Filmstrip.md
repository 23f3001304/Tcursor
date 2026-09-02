# src/editor/timeline/Filmstrip.tsx

The timeline clip track: a strip of evenly-spaced frame thumbnails (from `ensureThumbs`) filling the width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks pass through to the track body); a `Shimmer` loading skeleton shows until the thumbnails load.

## Filmstrip

```tsx
export const Filmstrip: React.MemoExoticComponent<(props: { thumbs: string[] }) => JSX.Element>
```

### Props

- `thumbs: string[]` - thumbnail asset URLs. Each renders as an `<img>` at `width: 100 / thumbs.length %` with `object-fit: cover` and `pointer-events: none`. An empty array renders `<Shimmer className="e-filmstrip" />` (`src/editor/timeline/Shimmer.tsx`) - same box (height/radius/background/border) as the loaded strip, so nothing jumps once thumbnails arrive. A real recording always eventually produces at least one frame, so an empty `thumbs` here is unambiguously "still loading," unlike `AudioTrack`'s `src`.

### Render hygiene

`React.memo`'d - `thumbs` is set once per project load and never changes on a playhead tick or an unrelated edit, so without `memo` this was re-rendering (and re-diffing all 16 `<img>`s) on every one of those for no reason.
