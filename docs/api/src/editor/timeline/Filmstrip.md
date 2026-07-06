# src/editor/timeline/Filmstrip.tsx

The timeline clip track: a strip of evenly-spaced frame thumbnails (from `ensureThumbs`) filling the width, so the timeline shows the recording's frames like Filmora. Non-interactive (seeks pass through to the track body); an empty well shows until the thumbnails load.

## Filmstrip

```tsx
export function Filmstrip({ thumbs }: { thumbs: string[] }): JSX.Element
```

### Props

- `thumbs: string[]` - thumbnail asset URLs. Each renders as an `<img>` at `width: 100 / thumbs.length %` with `object-fit: cover` and `pointer-events: none`. An empty array renders a plain `.e-filmstrip-empty` well.
