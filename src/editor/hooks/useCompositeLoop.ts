import { useEffect, useRef, type RefObject } from "react";
import type { CamSample, ClickSample, PreviewLayout, CursorKindSample, LayoutPresets } from "../../lib/ipc";
import type { ClickFxSettings, CursorSettings, ZoomSettings } from "../../hud/settings/settings";
import type { CameraMove, EffectRegion, LayoutSeg, Zoom } from "../../lib/edit";
import { camAt } from "../stage/camera";
import { type CamPose } from "../stage/cameraMoves";
import { activeCamDraft, frameCamLayout } from "../stage/frameCam";
import { drawPreview } from "../stage/previewCanvas";
import { requestFxOverlay, type FxCamRect } from "../stage/fxOverlay";
import { fxCamRect, fxFrameGeometry } from "../stage/fxGeometry";
import { drawMirroredRipples, overlayNeedsClicks } from "../stage/ripplePreview";
import { fxCacheKey, fxResponseAction, spotParamsKey, timeBucket } from "./fxCacheKey";
import { resolveSpotlight, newSpotlightSimState, spotAlphaPlan, type SpotlightSimState } from "../stage/spotlightPreview";
import { layoutAt } from "../timeline/layoutTrack";
import type { CursorSpritesState } from "./useCursorSprites";

// The FX overlay (spotlight + click effects) is a full IPC round-trip: shader render, a per-pixel
// alpha-reconstruct pass, PNG encode, base64, then a JS Image decode. The spotlight tracks the
// cursor, which moves nearly every frame during playback, so a request per rAF tick (the old
// behavior) meant that round-trip 60x/sec - the actual cause of the preview stutter. Two
// independent cuts: cap the request cadence well under 60fps (FX_BUCKET_MS, still visually smooth
// for a soft spotlight/ripple), and render at reduced resolution (FX_SCALE - soft gradients are
// invisible when upscaled), which shrinks the per-pixel loop, the PNG, and the decode all at once.
const FX_BUCKET_MS = 40; // ~25fps cap on backend FX-overlay requests
const FX_SCALE = 0.5; // internal render resolution factor vs the canvas; blit upscales automatically

/** Single rAF loop: reads the live time, keeps the webcam/audio roughly synced, composites the
 *  base frame (background + screen + webcam + cursor) via drawPreview, then blits the cached
 *  backend FX overlay (spotlight + click effects, rendered by the exact export shader pipeline)
 *  on top and kicks off a fresh overlay request whenever the FX-relevant state changes. */
export function useCompositeLoop({
  screenRef, webcamRef, audioRef, canvasRef,
  playRef, timeRef, onTimeRef,
  trackRef, layoutRef, layoutPresetsRef, layoutSegsRef, cameraMovesRef, zoomsRef, zoomSettingsRef, dragPoseRef, arrangingRef, clicksRef, effectsRef, clickfxRef, kindsRef, cursorRef,
  spritesRef, trailRef, dirtyRef, bgImgRef, spotSimRef,
}: {
  // The hidden media elements + the canvas they composite onto; then the clock (is it playing,
  // where is the playhead, and how to report it back).
  screenRef: RefObject<HTMLVideoElement | null>; webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>; canvasRef: RefObject<HTMLCanvasElement | null>;
  playRef: RefObject<boolean>; timeRef: RefObject<number>; onTimeRef: RefObject<(ms: number) => void>;
  trackRef: RefObject<CamSample[]>;
  layoutRef: RefObject<PreviewLayout | null>;
  layoutPresetsRef: RefObject<LayoutPresets | null>;
  layoutSegsRef: RefObject<LayoutSeg[]>;
  cameraMovesRef: RefObject<CameraMove[]>;
  zoomsRef: RefObject<Zoom[]>;
  zoomSettingsRef: RefObject<ZoomSettings>;
  dragPoseRef: RefObject<CamPose | null>;
  /** Arrange mode owns the stage: `activeCamDraft` suppresses the Move draft's EFFECT without
   *  clearing `dragPoseRef` - see `frameCam.md`. */ arrangingRef: RefObject<boolean>;
  clicksRef: RefObject<ClickSample[]>;
  effectsRef: RefObject<EffectRegion[]>;
  clickfxRef: RefObject<ClickFxSettings>;
  kindsRef: RefObject<CursorKindSample[]>;
  cursorRef: RefObject<CursorSettings>;
  spritesRef: RefObject<CursorSpritesState>;
  trailRef: RefObject<[number, number][]>;
  dirtyRef: RefObject<boolean>;
  bgImgRef: RefObject<HTMLImageElement | null>;
  spotSimRef: RefObject<SpotlightSimState>; // owned by Stage.tsx: also reset on a paused effects-content edit (gate 2, spotEffectsKeyRef)
}) {
  const lastReportRef = useRef(0);
  const lastFrameTRef = useRef(0);
  const fxOverlayImgRef = useRef<HTMLImageElement | null>(null);
  // Whether the CACHED overlay image may be faded client-side (see `spotAlphaPlan`): true only for
  // an image that was requested at a reference alpha of 1 and contains nothing but the spotlight.
  const fxSeparableRef = useRef(false);
  const fxInflightRef = useRef(false);
  const fxLastTRef = useRef(""); // key of the last response actually APPLIED
  const fxWantRef = useRef(""); // key computed on the MOST RECENT tick (fires or not)
  const offscreenRef = useRef<HTMLCanvasElement | null>(null);
  // `drawPreview`'s scratch layer for compositing a part-transparent panel in one go (see
  // `paintPanel`). Separate from `offscreenRef`, which holds the base scene the panel draws INTO;
  // only allocated/touched on the frames a layout transition is actually running.
  const layerRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const sv = screenRef.current, c = canvasRef.current, play = playRef.current;
      // Paused + nothing changed: skip compositing entirely so the editor isn't burning 60fps
      // redrawing a static frame (the idle/interaction-lag fix). Playback always composites.
      if (sv && c && (play || dirtyRef.current)) {
        dirtyRef.current = false;
        const t = play ? sv.currentTime * 1000 : timeRef.current;
        // Discontinuous jump (seek/re-sync), not natural playback advance - resets the trail AND
        // the spotlight sim (gate 1; gate 2 is Stage.tsx's spotEffectsKeyRef, for a paused edit).
        if (Math.abs(t - lastFrameTRef.current) > 200 || t < lastFrameTRef.current) { trailRef.current.length = 0; spotSimRef.current = newSpotlightSimState(); }
        lastFrameTRef.current = t;
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
            const baseLayout = layoutAt(layoutSegsRef.current, layoutPresetsRef.current, t, [c.width, c.height]) ?? layoutRef.current;
            // What drives the webcam PiP this frame (keyframe/drag override, else the smart
            // zoom action) - see frameCamLayout, which mirrors step_camera's ordering.
            const frameLayout = frameCamLayout(baseLayout, t, cam.scale, cameraMovesRef.current,
              activeCamDraft(dragPoseRef.current, arrangingRef.current), zoomsRef.current, zoomSettingsRef.current, c.width, c.height);
            // Draw the base frame (background + screen + webcam + cursor) WITHOUT FX
            if (!offscreenRef.current) offscreenRef.current = document.createElement("canvas");
            if (!layerRef.current) layerRef.current = document.createElement("canvas");
            drawPreview(ctx, c.width, c.height, sv, webcamRef.current, cam,
              frameLayout, bgImgRef.current, clicksRef.current, t, cur, offscreenRef.current,
              layerRef.current, layoutPresetsRef.current?.inset_w);
            // Panel/zoom mapping for the FX-overlay request AND the ripple draw below - mirrors
            // drawPreview's crop; see fxGeometry.ts for the math and why cw/ch are rounded.
            const { fxW, fxH, screenScale, map: mapFn } = fxFrameGeometry(c.width, c.height, frameLayout, cam, FX_SCALE);
            const cpos = mapFn(cam.curx, cam.cury);
            // Client-side click ripples (sweep-2, see ripplePreview.ts) - BEFORE the overlay blit
            // below, so an active spotlight's dim composites on top of it like everything else.
            drawMirroredRipples(ctx, clicksRef.current, t, cf.enabled, cf.style, cf.color, cf.intensity, mapFn, fxW, fxH, c.width, c.height);
            // resolveSpotlight runs EXACTLY once per tick (it advances spotSimRef in place), and now
            // runs BEFORE the blit rather than after it, because the blit needs this frame's alpha.
            const spot = { effects: effectsRef.current, on: cf.spotlight,
              params: { dim: cf.spotlight_dim, radius: cf.spotlight_radius, feather: cf.spotlight_feather,
                mode: cf.spotlight_mode, tint: cf.spotlight_tint } };
            const resolvedSpot = resolveSpotlight(spot, t, spotSimRef.current);
            // The spotlight's fade is applied HERE, at 60fps, instead of being re-rendered by the
            // backend at FX_BUCKET_MS - which is why it used to pop on/off instead of fading (a
            // 250ms fade got ~2 overlay updates, fewer when a request outlived it). `separable`
            // rides with the cached image, not with this tick's plan: a response can land after the
            // plan moved on, and the stored flag is the basis the CACHED pixels were rendered at.
            const plan = spotAlphaPlan(resolvedSpot, overlayNeedsClicks(cf.style));
            const fxImg = fxOverlayImgRef.current;
            const blitAlpha = fxSeparableRef.current ? plan.drawAlpha : 1;
            if (fxImg && fxImg.complete && fxImg.naturalWidth > 0 && blitAlpha > 0) {
              ctx.save();
              ctx.globalAlpha = blitAlpha;
              ctx.drawImage(fxImg, 0, 0, c.width, c.height);
              ctx.restore();
            }
            const camRect: FxCamRect = fxCamRect(frameLayout, fxW, fxH);
            const spotParamsStr = spotParamsKey(resolvedSpot, screenScale, plan.separable);
            const cursorStr = cpos ? `${Math.round(cpos[0])}-${Math.round(cpos[1])}` : "none";
            // "" for a mirrored style (ripplePreview.ts draws it instead) - the real click list
            // only for an unmirrored one, which still falls back to this overlay (see fxCacheKey.ts).
            const clicksStr = overlayNeedsClicks(cf.style) ? clicksRef.current.map(clk => `${clk.t}-${clk.x}-${clk.y}`).join(";") : "";
            const fxParamsStr = `${cf.style}-${cf.color.join(",")}-${cf.intensity}-${cf.enabled}-${cf.spotlight_dim_camera}`;
            const camStr = camRect ? camRect.rect.map(v => Math.round(v)).join(",") + `-${Math.round(camRect.radius)}` : "none";
            // cursorStr alone would invalidate the cache at full 60fps during playback - timeBucket
            // caps how often that's allowed to trigger a backend round-trip (see the file banner).
            const cacheKey = fxCacheKey(timeBucket(t, FX_BUCKET_MS), cursorStr, spotParamsStr, clicksStr, fxParamsStr, camStr);
            fxWantRef.current = cacheKey; // every tick, fired or not - lets `.then` detect staleness below

            if (!fxInflightRef.current && cacheKey !== fxLastTRef.current) {
              fxInflightRef.current = true;
              // Reuses the tick's ONE resolveSpotlight call (above) - resolving again would double-advance the sim.
              // `requestAlpha` is 1 on the separable path (a reference, alpha-independent overlay)
              // and the live alpha otherwise; `separable` is captured so the response latches the
              // basis its own pixels were rendered at, not whatever the plan says by then.
              const separable = plan.separable;
              const reqSpot = resolvedSpot ? { ...resolvedSpot, alpha: plan.requestAlpha } : null;
              requestFxOverlay(fxW, fxH, clicksRef.current, t, cpos, reqSpot, cf, mapFn, screenScale, camRect)
                .then(url => {
                  fxInflightRef.current = false;
                  const action = fxResponseAction(cacheKey, fxWantRef.current, url);
                  // Stale (wanted key moved on): drop it unlatched, next tick reissues for real.
                  if (action.kind === "stale") return;
                  // Any RESOLVED response - even a definite "off" -> `null`, the common no-fx
                  // case - is a valid terminal state: latch so the loop stops re-requesting until
                  // the key changes. Only a REJECTED request (`.catch` below) stays retryable.
                  fxLastTRef.current = cacheKey;
                  if (action.imageUrl) {
                    const img = new Image();
                    img.onload = () => { fxOverlayImgRef.current = img; fxSeparableRef.current = separable; dirtyRef.current = true; };
                    img.src = action.imageUrl;
                  } else {
                    fxOverlayImgRef.current = null; fxSeparableRef.current = false; dirtyRef.current = true; // nothing active at this key
                  }
                })
                .catch(() => { fxInflightRef.current = false; }); // failure: unlatched, retried
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
