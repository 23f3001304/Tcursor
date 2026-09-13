# src/editor/stage/stageCursor.ts

## stageCursor

```ts
export function stageCursor(cursor: CursorSettings, osCursorInVideo: boolean, cursorLayer: CursorLayerDto | null): { captured, plainOs, effCursor }
```

What the stage actually draws for the cursor setting, moved out of `Stage.tsx` (at the size cap) unchanged. "System" means the captured OS-cursor layer when the recording has one (`captured`); a recording that baked no cursor into its pixels and captured no layer has nothing original to show (`plainOs`), so it is redrawn as a plain Enhanced pointer with no bounce and no trail (`effCursor`), and `Stage` drops the cursor-kind track so the sprite never changes shape.
