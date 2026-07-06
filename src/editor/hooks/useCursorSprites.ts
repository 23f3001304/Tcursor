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

  useEffect(() => {
    const sprites = new Map<string, HTMLImageElement>();
    const hots = new Map<string, [number, number]>();
    const canvasH = new Map<string, number>();

    for (const s of cursorSprites) {
      const img = new Image();
      img.onload = onLoaded;
      img.src = s.url;
      sprites.set(s.kind, img);
      hots.set(s.kind, s.hot);
      canvasH.set(s.kind, s.canvas_h);
    }

    spritesRef.current = { sprites, hots, canvasH };
  }, [cursorSprites, onLoaded]);

  return spritesRef;
}
