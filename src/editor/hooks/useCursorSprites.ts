import { useEffect, useRef } from "react";
import type { CursorSpriteDto } from "../../lib/ipc";

export interface CursorSpritesState {
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
}

export function useCursorSprites(
  cursorSprites: CursorSpriteDto[],
  onLoaded: () => void
) {
  const spritesRef = useRef<CursorSpritesState>({
    sprites: new Map(),
    hots: new Map(),
    canvasH: new Map(),
  });
  // Callers (Stage) pass an inline `() => { dirtyRef.current = true; }` that's a new function
  // identity every render - kept in a ref (not the effect's own dep array) so the decode effect
  // below only re-runs when `cursorSprites` itself actually changes, not on every unrelated
  // Stage render (it used to re-decode every sprite ~16x/sec during playback).
  const onLoadedRef = useRef(onLoaded);
  onLoadedRef.current = onLoaded;

  useEffect(() => {
    const sprites = new Map<string, HTMLImageElement>();
    const hots = new Map<string, [number, number]>();
    const canvasH = new Map<string, number>();

    for (const s of cursorSprites) {
      const img = new Image();
      img.onload = () => onLoadedRef.current();
      img.src = s.url;
      sprites.set(s.kind, img);
      hots.set(s.kind, s.hot);
      canvasH.set(s.kind, s.canvas_h);
    }

    spritesRef.current = { sprites, hots, canvasH };
  }, [cursorSprites]);

  return spritesRef;
}
