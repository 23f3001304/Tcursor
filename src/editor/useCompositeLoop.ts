import { useEffect, useRef, type RefObject } from "react";
import type { CamSample, ClickSample, PreviewLayout, CursorKindSample, LayoutPresets } from "../lib/ipc";
import type { ClickFxSettings, CursorSettings } from "../hud/settings";
import type { EffectRegion, LayoutSeg } from "../lib/edit";
import { camAt } from "./camera";
import { drawPreview } from "./previewCanvas";
import { requestFxOverlay } from "./fxOverlay";
import { resolveSpotlight, newSpotlightSimState } from "./spotlightPreview";
import { layoutAt } from "./layoutTrack";
import type { CursorSpritesState } from "./useCursorSprites";

// The FX overlay (spotlight + click effects) is a full IPC round-trip: shader render, a per-pixel
// alpha-reconstruct pass, PNG encode, base64, then a JS Image decode. The spotlight tracks the
// cursor, which moves on nearly every frame during normal playback, so a request per rAF tick (the
// old behavior) meant that round-trip 60x/sec - the actual cause of the preview stutter. Two
// independent cuts, since they attack different parts of the cost:
//  - cap the request cadence well under 60fps (still visually smooth for a soft spotlight/ripple)
//  - render it at reduced resolution (the effects are soft gradients, invisible when upscaled),
//    which shrinks the per-pixel loop, the PNG, and the decode all at once
const FX_BUCKET_MS = 40; // ~25fps cap on backend FX-overlay requests
const FX_SCALE = 0.5; // internal render resolution factor vs the canvas; blit upscales automatically

/** Single rAF loop: reads the live time, keeps the webcam/audio roughly synced, composites the
 *  base frame (background + screen + webcam + cursor) via drawPreview, then blits the cached
 *  backend FX overlay (spotlight + click effects, rendered by the exact export shader pipeline)
 *  on top and kicks off a fresh overlay request whenever the FX-relevant state changes. */
export function useCompositeLoop({
  screenRef, webcamRef, audioRef, canvasRef,
  playRef, timeRef, onTimeRef,
  trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  spritesRef, trailRef, dirtyRef, bgImgRef,
}: {
  screenRef: RefObject<HTMLVideoElement | null>;
  webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>;
  canvasRef: RefObject<HTMLCanvasElement | null>;
  playRef: RefObject<boolean>;
  timeRef: RefObject<number>;
  onTimeRef: RefObject<(ms: number) => void>;
  trackRef: RefObject<CamSample[]>;
  layoutRef: RefObject<PreviewLayout | null>;
  layoutPresetsRef: RefObject<LayoutPresets | null>;
  layoutSegsRef: RefObject<LayoutSeg[]>;
  clicksRef: RefObject<ClickSample[]>;
  effectsRef: RefObject<EffectRegion[]>;
  clickfxRef: RefObject<ClickFxSettings>;
  kindsRef: RefObject<CursorKindSample[]>;
  cursorRef: RefObject<CursorSettings>;
  spritesRef: RefObject<CursorSpritesState>;
  trailRef: RefObject<[number, number][]>;
  dirtyRef: RefObject<boolean>;
  bgImgRef: RefObject<HTMLImageElement | null>;
}) {
  const lastReportRef = useRef(0);
  const fxOverlayImgRef = useRef<HTMLImageElement | null>(null);
  const fxInflightRef = useRef(false);
  const fxLastTRef = useRef("");
  const offscreenRef = useRef<HTMLCanvasElement | null>(null);
  const spotSimRef = useRef(newSpotlightSimState());

  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const sv = screenRef.current, c = canvasRef.current, play = playRef.current;
      // Paused + nothing changed: skip compositing entirely so the editor isn't burning 60fps
      // redrawing a static frame (the idle/interaction-lag fix). Playback always composites.
      if (sv && c && (play || dirtyRef.current)) {
        dirtyRef.current = false;
        const t = play ? sv.currentTime * 1000 : timeRef.current;
        if (play) {
          // Throttle the React state update to ~16fps - it re-renders the whole editor tree. The
          // canvas itself stays 60fps because it reads sv.currentTime directly, not this state.
          if (t < lastReportRef.current || t - lastReportRef.current >= 60) { onTimeRef.current(t); lastReportRef.current = t; }
          const wv = webcamRef.current, av = audioRef.current;
          if (wv && Math.abs(wv.currentTime - sv.currentTime) > 0.15) wv.currentTime = sv.currentTime;
          if (av && Math.abs(av.currentTime - sv.currentTime) > 0.18) av.currentTime = sv.currentTime;
        }
        const ctx = c.getContext("2d");
        if (ctx) {
          try {
            const sp = spritesRef.current, cs = cursorRef.current, cf = clickfxRef.current;
            const cam = camAt(trackRef.current, t);
            const cur = { style: cs.style, size: cs.size, clickBounce: cs.click_bounce, bounceIntensity: cs.bounce_intensity,
              motionBlur: cs.motion_blur, kinds: kindsRef.current, sprites: sp.sprites, hots: sp.hots, canvasH: sp.canvasH, recent: trailRef.current };
            // The active layout at this exact frame time (cross-faded across a layout-segment
            // boundary); falls back to the static layout when presets haven't loaded or there
            // are no segments. Used for both the base draw below and the FX screen-rect math.
            const frameLayout = layoutAt(layoutSegsRef.current, layoutPresetsRef.current, t) ?? layoutRef.current;
            // Draw the base frame (background + screen + webcam + cursor) WITHOUT FX
            if (!offscreenRef.current) offscreenRef.current = document.createElement("canvas");
            drawPreview(ctx, c.width, c.height, sv, webcamRef.current, cam,
              frameLayout, bgImgRef.current, clicksRef.current, t, cur, offscreenRef.current);
            // Blit the cached backend FX overlay on top
            const fxImg = fxOverlayImgRef.current;
            if (fxImg && fxImg.complete && fxImg.naturalWidth > 0) {
              ctx.drawImage(fxImg, 0, 0, c.width, c.height);
            }
            // Request a fresh FX overlay from the backend (async, non-blocking), rendered at
            // FX_SCALE resolution - the mapping below targets that smaller canvas directly so the
            // hit/cursor coordinates already line up with what the backend renders. This mirrors
            // drawPreview's whole-frame zoom crop (coordmap::crop in the export) so spotlight/click
            // positions land exactly where the zoomed base frame puts them, not where a crop of the
            // raw screen source alone would.
            const fxW = Math.max(1, Math.round(c.width * FX_SCALE)), fxH = Math.max(1, Math.round(c.height * FX_SCALE));
            const lay = frameLayout, pad = Math.min(fxW, fxH) * 0.045;
            const dx = lay ? lay.screen[0] * fxW : pad, dy = lay ? lay.screen[1] * fxH : pad;
            const dw = lay ? lay.screen[2] * fxW : fxW - 2 * pad, dh = lay ? lay.screen[3] * fxH : fxH - 2 * pad;
            // Spotlight radius/feather are fractions of the SCREEN panel, not the whole FX frame -
            // pre-scale by the panel's height fraction so the preview matches the export's
            // screen-panel-relative spotlight (fx_state.rs). See requestFxOverlay.
            const screenScale = fxH > 0 ? dh / fxH : 1;
            const scale = Math.max(cam.scale, 0.01);
            const cw = Math.max(1, fxW / scale), ch = Math.max(1, fxH / scale);
            const camPxX = dx + cam.cx * dw, camPxY = dy + cam.cy * dh;
            const cx0 = Math.min(Math.max(camPxX - cw / 2, 0), Math.max(0, fxW - cw));
            const cy0 = Math.min(Math.max(camPxY - ch / 2, 0), Math.max(0, fxH - ch));
            const mapFn = (fx: number, fy: number): [number, number] | null => {
              const bx = dx + fx * dw, by = dy + fy * dh; // panel-local point, pre-zoom
              return [(bx - cx0) * fxW / cw, (by - cy0) * fxH / ch];
            };
            const cpos = mapFn(cam.curx, cam.cury);
            const spot = { effects: effectsRef.current, on: cf.spotlight,
              params: { dim: cf.spotlight_dim, radius: cf.spotlight_radius, feather: cf.spotlight_feather,
                mode: cf.spotlight_mode, tint: cf.spotlight_tint } };

            // Built from the RESOLVED spotlight (region overrides applied), not the raw global
            // settings - otherwise editing a region's dim/radius/feather/mode in the inspector
            // never changed this string, so the cache key never invalidated, so the new look
            // only ever showed up once something else (cursor movement, a time bucket change)
            // coincidentally forced a fresh request. That's why it required scrubbing to see.
            const resolvedSpot = resolveSpotlight(spot, t, spotSimRef.current);
            const spotParamsStr = resolvedSpot
              ? `${resolvedSpot.dim}-${resolvedSpot.radius}-${resolvedSpot.feather}-${resolvedSpot.mode}-${resolvedSpot.tint.join(",")}-${screenScale.toFixed(3)}`
              : "off";
            const clicksStr = clicksRef.current.map(clk => `${clk.t}-${clk.x}-${clk.y}`).join(";");
            const cursorStr = cpos ? `${Math.round(cpos[0])}-${Math.round(cpos[1])}` : "none";
            const fxParamsStr = `${cf.style}-${cf.color.join(",")}-${cf.intensity}-${cf.enabled}`;
            // The spotlight tracks the cursor, which moves almost every frame during playback, so
            // cursorStr alone would invalidate the cache at full 60fps regardless of anything else.
            // Bucket time to a fixed cadence to cap how often that's allowed to trigger a backend
            // round-trip; the last rendered overlay stays on screen between updates.
            const tBucket = Math.round(t / FX_BUCKET_MS) * FX_BUCKET_MS;
            const cacheKey = `${tBucket}_${cursorStr}_${spotParamsStr}_${clicksStr}_${fxParamsStr}`;

            if (!fxInflightRef.current && cacheKey !== fxLastTRef.current) {
              fxInflightRef.current = true;
              fxLastTRef.current = cacheKey;
              requestFxOverlay(fxW, fxH, clicksRef.current, t, cpos, spot, cf, mapFn, spotSimRef.current, screenScale)
                .then(url => {
                  fxInflightRef.current = false;
                  if (url) {
                    const img = new Image();
                    img.onload = () => { fxOverlayImgRef.current = img; dirtyRef.current = true; };
                    img.src = url;
                  } else {
                    fxOverlayImgRef.current = null; dirtyRef.current = true;
                  }
                })
                .catch(() => { fxInflightRef.current = false; });
            }
          } catch (e) { if (import.meta.env.DEV) console.error("drawPreview", e); }
        }
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // refs only: identities are stable across renders, so the loop always reads live values.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
