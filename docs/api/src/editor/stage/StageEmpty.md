# src/editor/stage/StageEmpty.tsx

## StageEmpty

```tsx
export function StageEmpty(): JSX.Element
```

The stage before the preview proxy exists: the wave motif's horizon line with the brand dot landing on it, over the one line of copy that says what is happening ("Preparing preview").

### Why it is its own component

`Stage.tsx` sits at its line cap, and this replaced a one-line inline branch that rendered a `Spin` beside the text. Stage still renders one element.

### Design note

This is the editor's only empty state, and the panel-design benchmark calls out (item (c) 6) that every competitor's empty state is inert - a motion-led one is unclaimed ground. The generic loader ring that used to sit here is exactly the "cheap tell" the benchmark's section (e) warns about at the moment the brand should feel most in control.

### Used by

- `src/editor/stage/Stage.tsx` - rendered when there is no `src` and no error.
