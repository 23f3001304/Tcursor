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
- `onLoaded: () => void` - called on each sprite's `img.onload`; `Stage.tsx` passes an inline `() => { dirtyRef.current = true; }` that marks the paused frame dirty so a late-decoding sprite still gets composited. A NEW function identity every `Stage` render - kept in a ref internally (`onLoadedRef`, reassigned every render, read inside `img.onload`) rather than in the decode effect's own dependency array, so it does not affect when sprites actually re-decode (see Implementation).

### Returns

`RefObject<CursorSpritesState>` - starts as empty maps, replaced wholesale each time `cursorSprites` changes.

### Implementation

The decode effect's dependency array is `[cursorSprites]` ONLY - not `onLoaded`. Before this, `onLoaded` (Stage's inline, per-render callback) was also a dependency, so the effect re-ran - re-creating every `Image`, re-triggering every decode - on every Stage render, which during playback (a `timeMs` state update every frame) meant sprites were re-decoding roughly 16x/sec instead of only when the sprite pack itself changed (`doc.settings.cursor.pack`). On `cursorSprites` change: build fresh `Map`s, create one `Image` per DTO with `img.onload = () => onLoadedRef.current()` (always calls whatever the LATEST `onLoaded` is, via the ref), `img.src = dto.url`, and assign all three maps into the ref in one shot (not incrementally per sprite, so the loop never reads a half-populated set of maps).
