import type { RefObject } from "react";
import { camAt } from "../../stage/camera/camera";
import { activeCamDraft, frameCamLayout } from "../../stage/camera/frameCam";
import { drawPreview } from "../../stage/canvas/previewCanvas";
import { fxFrameGeometry } from "../../stage/fx/fxGeometry";
import { drawMirroredRipples, overlayNeedsClicks } from "../../stage/fx/ripplePreview";
import { drawCaptions } from "../../stage/fx/captionDraw";
import { resolveSpotlight, spotAlphaPlan } from "../../stage/fx/spotlightPreview";
import { layoutAt } from "../../timeline/model/layoutTrack";
import { newTilt, tiltFromCam, type TiltState } from "../../stage/cursor/cursorTilt";
import { fxRequestTick } from "./fxRequestTick";
import { exactKey } from "./useExactFrame";
import type { CompositeLoopRefs } from "./compositeLoopRefs";

const FX_SCALE = 0.5;

export interface FrameScratch {
  fxOverlayImgRef: RefObject<HTMLImageElement | null>;
  fxSeparableRef: RefObject<boolean>;
  fxInflightRef: RefObject<boolean>;
  fxLastTRef: RefObject<string>;
  fxWantRef: RefObject<string>;
  offscreenRef: RefObject<HTMLCanvasElement | null>;
  layerRef: RefObject<HTMLCanvasElement | null>;
  tiltRef: RefObject<TiltState>;
}

export function newFrameScratch(): FrameScratch {
  return {
    fxOverlayImgRef: { current: null },
    fxSeparableRef: { current: false },
    fxInflightRef: { current: false },
    fxLastTRef: { current: "" },
    fxWantRef: { current: "" },
    offscreenRef: { current: null },
    layerRef: { current: null },
    tiltRef: { current: newTilt() },
  };
}

export function drawCompositeFrame(
  ctx: CanvasRenderingContext2D,
  c: HTMLCanvasElement,
  sv: HTMLVideoElement,
  r: CompositeLoopRefs,
  s: FrameScratch,
  play: boolean,
  t: number,
  tOut: number,
  dtOut: number,
) {
  try {
    const sp = r.spritesRef.current,
      cs = r.cursorRef.current,
      cf = r.clickfxRef.current;
    const cam = camAt(r.trackRef.current, tOut);
    const cur = {
      style: cs.style,
      size: cs.size,
      clickBounce: cs.click_bounce,
      bounceIntensity: cs.bounce_intensity,
      motionBlur: cs.motion_blur,
      kinds: r.kindsRef.current,
      sprites: sp.sprites,
      hots: sp.hots,
      canvasH: sp.canvasH,
      captured: sp.captured,
      busy: sp.busy,
      busyFrames: sp.busyFrames,
      material: sp.material,
      back: cs.back,
      tiltDeg: 0,
      recent: r.trailRef.current,
    };
    const baseLayout =
      layoutAt(r.layoutSegsRef.current, r.layoutPresetsRef.current, tOut, [c.width, c.height]) ??
      r.layoutRef.current;
    const frameLayout = frameCamLayout(
      baseLayout,
      tOut,
      cam.scale,
      r.cameraMovesRef.current,
      activeCamDraft(r.dragPoseRef.current, r.arrangingRef.current),
      r.zoomsRef.current,
      r.zoomSettingsRef.current,
      c.width,
      c.height,
    );
    const sr = frameLayout?.screen;
    const aspect = sr && sr[3] > 0 ? (sr[2] * c.width) / (sr[3] * c.height) : 16 / 9;
    cur.tiltDeg = tiltFromCam(s.tiltRef.current, cam.curx, cam.cury, aspect, dtOut, cs.tilt);
    if (!s.offscreenRef.current) s.offscreenRef.current = document.createElement("canvas");
    if (!s.layerRef.current) s.layerRef.current = document.createElement("canvas");
    drawPreview(
      ctx,
      c.width,
      c.height,
      sv,
      r.webcamRef.current,
      cam,
      frameLayout,
      r.bgRef.current,
      r.clicksRef.current,
      t,
      cur,
      s.offscreenRef.current,
      s.layerRef.current,
      r.layoutPresetsRef.current?.inset_w,
      tOut,
    );
    const {
      fxW,
      fxH,
      screenScale,
      map: mapFn,
      mapCanvas,
    } = fxFrameGeometry(c.width, c.height, frameLayout, cam, FX_SCALE);
    const cpos = mapFn(cam.curx, cam.cury);
    drawMirroredRipples(
      ctx,
      r.clicksRef.current,
      t,
      cf.enabled,
      cf.style,
      cf.color,
      cf.intensity,
      mapCanvas,
      fxW,
      fxH,
      c.width,
      c.height,
    );
    const spot = {
      effects: r.effectsRef.current,
      on: cf.spotlight,
      params: {
        dim: cf.spotlight_dim,
        radius: cf.spotlight_radius,
        feather: cf.spotlight_feather,
        mode: cf.spotlight_mode,
        tint: cf.spotlight_tint,
      },
    };
    const resolvedSpot = resolveSpotlight(spot, tOut, r.spotSimRef.current);
    const plan = spotAlphaPlan(resolvedSpot, overlayNeedsClicks(cf.style));
    const fxImg = s.fxOverlayImgRef.current;
    const blitAlpha = s.fxSeparableRef.current ? plan.drawAlpha : 1;
    if (fxImg && fxImg.complete && fxImg.naturalWidth > 0 && blitAlpha > 0) {
      ctx.save();
      ctx.globalAlpha = blitAlpha;
      ctx.drawImage(fxImg, 0, 0, c.width, c.height);
      ctx.restore();
    }
    drawCaptions(
      ctx,
      c.width,
      c.height,
      r.captionsRef.current,
      r.capStyleRef.current,
      r.accentRef.current,
      tOut,
    );
    fxRequestTick(
      {
        fxLastTRef: s.fxLastTRef,
        fxWantRef: s.fxWantRef,
        fxInflightRef: s.fxInflightRef,
        fxOverlayImgRef: s.fxOverlayImgRef,
        fxSeparableRef: s.fxSeparableRef,
        dirtyRef: r.dirtyRef,
      },
      {
        frameLayout,
        fxW,
        fxH,
        screenScale,
        mapFn: mapCanvas,
        cpos,
        resolvedSpot,
        plan,
        cf,
        clicks: r.clicksRef.current,
        t,
      },
    );
    const ex = r.exactRef.current;
    if (!play && ex && ex.key === exactKey(t, r.editGenRef.current))
      ctx.drawImage(ex.img, 0, 0, c.width, c.height);
  } catch (e) {
    if (import.meta.env.DEV) console.error("drawPreview", e);
  }
}
