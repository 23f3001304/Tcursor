# src/editor/stage/stageCursor.ts

## stageCursor

```ts
export function stageCursor(cursor: CursorSettings, osCursorInVideo: boolean, cursorLayer: CursorLayerDto | null): { captured, plainOs, effCursor }
```

What the stage actually draws for the cursor setting, moved out of `Stage.tsx` (at the size cap) unchanged. "System" means the captured OS-cursor layer when the recording has one (`captured`); a recording that baked no cursor into its pixels and captured no layer has nothing original to show (`plainOs`), so it is redrawn as a plain Enhanced pointer with no bounce, no trail, no motion tilt and no glass back (`effCursor`), and `Stage` drops the cursor-kind track so the sprite never changes shape. That list is the preview's half of Rust `CursorSettings::plain_os` - the same "no fake polish a real OS cursor does not have" rule `smoothness_at` / `idealize_at` / `tilt_at` apply on the export side, so anything added to one must be added here too.
