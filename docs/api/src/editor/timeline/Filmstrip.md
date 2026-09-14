# src/editor/timeline/Filmstrip.tsx

The timeline clip track: a strip of evenly-spaced frame thumbnails (from `ensureThumbs`) filling the width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks pass through to the track body); a `Shimmer` loading skeleton shows until the thumbnails load.

## Filmstrip

```tsx
export const Filmstrip: React.MemoExoticComponent<(props: { thumbs: string[] }) => JSX.Element>
```

### Props

- `thumbs: string[]` - thumbnail asset URLs. Each renders as an `<img>` at `width: 100 / thumbs.length %` with `object-fit: cover` and `pointer-events: none`. An empty array renders `<Shimmer className="e-filmstrip" />` (`src/editor/timeline/Shimmer.tsx`) - same box (height/radius/background/border) as the loaded strip, so nothing jumps once thumbnails arrive. A real recording always eventually produces at least one frame, so an empty `thumbs` here is unambiguously "still loading," unlike `AudioTrack`'s `src`.

### Look (timeline-readability pass, owner: "the thumbnails are extremely small and repetitive")

The lane is 80px tall, 1.5x the 54px it was, and `useEditorData` asks for `FILMSTRIP_COUNT` (9) thumbnails generated at `FILMSTRIP_HEIGHT` (80) rather than sixteen at a fixed 64 - so a tile is ~148px wide, within a few percent of its native 16:9 shape, and is never an upscaled smaller JPEG. See `filmstripPlan.md` for how both numbers are derived and which Rust constants have to agree with them. `timeline.css` additionally rules a 1px `--e-bg` line between tiles (`border-box`, so the strip still totals exactly 100%): without it a slow-moving recording's tiles smear into one long image, which is the other half of "repetitive".

### Render hygiene

`React.memo`'d - `thumbs` is set once per project load and never changes on a playhead tick or an unrelated edit, so without `memo` this was re-rendering (and re-diffing every `<img>`) on every one of those for no reason.
