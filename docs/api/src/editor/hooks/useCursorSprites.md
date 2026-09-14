# src/editor/hooks/useCursorSprites.ts

Decodes the editor preview's cursor images into `<img>` elements once per change, exposed as a ref so the `rAF` compositing loop reads them without triggering React re-renders as each one finishes decoding. Two independent sets: the synthetic sprite pack (from `cursorSprites`) and the recording's captured OS-cursor layer (from `cursorLayer`).

## CursorSpritesState

```ts
export interface CursorSpritesState {
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  busy: BusySpec | null;
  busyFrames: HTMLImageElement[];
  captured: CapturedLayer | null;
}
```

Per cursor-kind lowercase name: the decoded sprite image, its hotspot (0..1 of the sprite), and its original canvas height (for uniform scaling). Plus `busy`/`busyFrames` - the selected pack's busy animation (pack format v2) and its decoded explicit frames, empty unless the pack ships them - and `captured`, the decoded real OS-cursor layer (images, hotspots, `[t, id]` track and the recording's own `srcW`), or `null`. Exactly what `DrawCursor` (`cursorPreview.ts`) needs, which is why `useCompositeLoop` can spread it straight into the per-frame cursor object.

### CursorSpritesState::material

```ts
material: string | null;
```

The selected pack's `material` (`"glass"`, or `null`) straight off the `cursor_sprites` DTO.

Decoded on the SPRITE effect, not the captured-layer one, because it is a property of the pack: it changes when the user picks a different pack, and must not cost a re-decode of the recording's real cursor bitmaps when it does.

`useCompositeLoop` passes it into `DrawCursor.material`, which is what makes the live canvas draw a lens pack's sprite at the same alpha the export blits it at.

## useCursorSprites

```ts
export function useCursorSprites(
  cursorPack: CursorPackDto | null, cursorLayer: CursorLayerDto | null, onLoaded: () => void
): RefObject<CursorSpritesState>
```

### Inputs

- `cursorPack: CursorPackDto | null` - the selected pack from the backend: one sprite DTO per kind (PNG data URL, hotspot, canvas height), its explicit busy frames, and its declared busy animation. `null` before the fetch resolves.
- `cursorLayer: CursorLayerDto | null` - the recording's captured OS-cursor layer, or `null`. `Stage` passes `null` unless the doc's cursor style is `"system"`, so this argument doubles as the "draw the real cursor" gate.
- `onLoaded: () => void` - called on each sprite's `img.onload`; `Stage.tsx` passes an inline `() => { dirtyRef.current = true; }` that marks the paused frame dirty so a late-decoding sprite still gets composited. A NEW function identity every `Stage` render - kept in a ref internally (`onLoadedRef`, reassigned every render, read inside `img.onload`) rather than in the decode effect's own dependency array, so it does not affect when sprites actually re-decode (see Implementation).

### Returns

`RefObject<CursorSpritesState>` - starts as empty maps and a `null` layer; each half is replaced wholesale when its own input changes.

### Implementation

The decode effect's dependency array is `[cursorPack]` ONLY - not `onLoaded`. Before this, `onLoaded` (Stage's inline, per-render callback) was also a dependency, so the effect re-ran - re-creating every `Image`, re-triggering every decode - on every Stage render, which during playback (a `timeMs` state update every frame) meant sprites were re-decoding roughly 16x/sec instead of only when the sprite pack itself changed (`doc.settings.cursor.pack`). On `cursorPack` change: build fresh `Map`s, create one `Image` per DTO (and one per explicit busy frame) with `img.onload = () => onLoadedRef.current()` (always calls whatever the LATEST `onLoaded` is, via the ref), `img.src = dto.url`, and assign all three maps into the ref in one shot (not incrementally per sprite, so the loop never reads a half-populated set of maps).

The captured layer decodes on its OWN effect, keyed `[cursorLayer]`. *Why not the same effect:* the layer is a property of the RECORDING (fetched once per folder) while the sprite pack re-decodes whenever the selected pack changes, so sharing one effect would re-decode every real cursor bitmap on an unrelated pack switch - and vice versa. Both effects merge into the existing ref value (`{ ...spritesRef.current, ... }`) rather than replacing it, so neither wipes the other's half.

