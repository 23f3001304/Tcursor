import { useEffect, useRef } from "react";
import type { CursorPackDto, CursorLayerDto } from "../../lib/ipc";
import type { CapturedLayer } from "../stage/cursorPreview";
import type { BusySpec } from "../stage/cursorBusy";

export interface CursorSpritesState {
  sprites: Map<string, HTMLImageElement>;
  hots: Map<string, [number, number]>;
  canvasH: Map<string, number>;
  /** The selected pack's busy animation (pack format v2), null for a still one. */
  busy: BusySpec | null;
  /** Decoded `busy_NN.png` frames when the pack ships them, empty otherwise. */
  busyFrames: HTMLImageElement[];
  /** The pack's `material` (`"glass"` or null) - what the preview needs to draw a lens pack's
   *  sprite at the same alpha the export blits it at. */
  material: string | null;
  /** The recording's captured OS-cursor layer, decoded - null when it has none, or when the
   *  live style is not "system" (the caller decides; see Stage). */
  captured: CapturedLayer | null;
}

export function useCursorSprites(
  cursorPack: CursorPackDto | null,
  cursorLayer: CursorLayerDto | null,
  onLoaded: () => void
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
    // Explicit busy frames ARE the animation; the declared `anim` only applies without them.
    const busyFrames = (cursorPack?.busy_frames ?? []).map((f) => decode(f.url));
    const busy = cursorPack?.busy ?? null;

    const material = cursorPack?.material ?? null;

    spritesRef.current = { ...spritesRef.current, sprites, hots, canvasH, busy, busyFrames, material };
  }, [cursorPack]);

  // The captured layer decodes on its OWN effect: it is a property of the recording (fetched once
  // per folder) while the sprite pack above re-decodes whenever the selected pack changes, so
  // sharing one effect would re-decode every real cursor bitmap on an unrelated pack switch.
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
