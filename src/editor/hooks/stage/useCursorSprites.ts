import { useEffect, useRef } from "react";
import type { CursorPackDto, CursorLayerDto } from "../../../shared/ipc";
import type { CapturedLayer } from "../../stage/cursor/cursorPreview";
import type { BusySpec } from "../../stage/cursor/cursorBusy";

export interface CursorSpritesState {
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  busy: BusySpec | null;
  busyFrames: HTMLImageElement[];
  material: string | null;
  captured: CapturedLayer | null;
}

export function useCursorSprites(
  cursorPack: CursorPackDto | null,
  cursorLayer: CursorLayerDto | null,
  onLoaded: () => void,
) {
  const spritesRef = useRef<CursorSpritesState>({
    sprites: new Map(),
    hots: new Map(),
    canvasH: new Map(),
    busy: null,
    busyFrames: [],
    material: null,
    captured: null,
  });
  const onLoadedRef = useRef(onLoaded);
  onLoadedRef.current = onLoaded;

  useEffect(() => {
    const sprites = new Map<string, HTMLImageElement>();
    const hots = new Map<string, [number, number]>();
    const canvasH = new Map<string, number>();
    const decode = (url: string) => {
      const img = new Image();
      img.onload = () => onLoadedRef.current();
      img.src = url;
      return img;
    };

    for (const s of cursorPack?.sprites ?? []) {
      sprites.set(s.kind, decode(s.url));
      hots.set(s.kind, s.hot);
      canvasH.set(s.kind, s.canvas_h);
    }
    const busyFrames = (cursorPack?.busy_frames ?? []).map((f) => decode(f.url));
    const busy = cursorPack?.busy ?? null;

    const material = cursorPack?.material ?? null;

    spritesRef.current = { ...spritesRef.current, sprites, hots, canvasH, busy, busyFrames, material };
  }, [cursorPack]);

  useEffect(() => {
    let captured: CapturedLayer | null = null;
    if (cursorLayer && cursorLayer.cursors.length) {
      const images = new Map<number, HTMLImageElement>();
      const hots = new Map<number, [number, number]>();
      for (const c of cursorLayer.cursors) {
        const img = new Image();
        img.onload = () => onLoadedRef.current();
        img.src = c.url;
        images.set(c.id, img);
        hots.set(c.id, [c.hx, c.hy]);
      }
      captured = { track: cursorLayer.track, images, hots, srcW: cursorLayer.src_w };
    }
    spritesRef.current = { ...spritesRef.current, captured };
  }, [cursorLayer]);

  return spritesRef;
}
