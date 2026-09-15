# src/editor/timeline/lanes/Shimmer.tsx

A quiet "still loading" skeleton reused by `Filmstrip` and `AudioTrack` while their data is still fetching.

## Shimmer

```tsx
export function Shimmer({ className }: { className?: string }): JSX.Element
```

### Props

- `className?: string` - the caller's own box class (`"e-filmstrip"` or `"e-audiorow"`), so the skeleton is pixel-identical in height/radius/background/border to the loaded content it stands in for - nothing jumps once real content replaces it.

### Behavior

Renders an `.e-shimmer` box (adds `position: relative; overflow: hidden` on top of whatever `className` supplies) containing one `motion.div.e-shimmer-sweep` - a 40%-wide `rgba(255,255,255,.04)` gradient band animated across the box via Motion's `x` (`["-100%", "350%"]`, a `1.6s` linear tween, `repeat: Infinity`). Per the fake-polish philosophy (every feel knob is an intentional, designed motion) and the Global Constraints, this is deliberately a Motion-animated layer, not a CSS `@keyframes` loop.

### Used by

`Filmstrip` (`className="e-filmstrip"`, while `thumbs` is empty) and `AudioTrack` (`className="e-audiorow"`, while `loading` and `src` is empty) - both in `src/editor/timeline/`.
