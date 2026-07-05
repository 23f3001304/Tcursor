# src/editor/StageToolbar.tsx

The Stage's frame-tool strip: four icon buttons overlaid on the preview corner.

## StageToolbar

```tsx
export function StageToolbar(): JSX.Element
```

Renders `.e-ftool` with four icon buttons (aspect ratio, cursor, captions, 3D camera) - decorative stubs, not yet wired to any action. Extracted from `Stage.tsx` to keep that file under the size limit; has no props or state.
