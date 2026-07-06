# src/editor/hooks/useCursorSprites.ts

Decodes the Capitaine cursor sprite pack (from the backend `cursorSprites` command) into `<img>` elements once per change, exposed as a ref so the `rAF` compositing loop reads them without triggering React re-renders as each sprite finishes decoding.

## CursorSpritesState

```ts
export interface CursorSpritesState {
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
}
```

Per cursor-kind lowercase name: the decoded sprite image, its hotspot (0..1 of the sprite), and its original canvas height (for uniform scaling) - exactly what `DrawCursor` (`cursorPreview.ts`) needs.

## useCursorSprites

```ts
export function useCursorSprites(
  cursorSprites: CursorSpriteDto[], onLoaded: () => void
): RefObject<CursorSpritesState>
```

### Inputs

- `cursorSprites: CursorSpriteDto[]` - the sprite pack DTOs (kind, PNG data URL, hotspot, canvas height) from the backend.
- `onLoaded: () => void` - called on each sprite's `img.onload`; `Stage.tsx` passes a callback that marks the paused frame dirty so a late-decoding sprite still gets composited.

### Returns

`RefObject<CursorSpritesState>` - starts as empty maps, replaced wholesale each time `cursorSprites` changes.

### Implementation

On `cursorSprites` change: build fresh `Map`s, create one `Image` per DTO with `img.onload = onLoaded`, `img.src = dto.url`, and assign all three maps into the ref in one shot (not incrementally per sprite, so the loop never reads a half-populated set of maps).
