import type { RefObject } from "react";
import { camAt } from "../../stage/camera/camera";
import { activeCamDraft, frameCamLayout } from "../../stage/camera/frameCam";
import { drawCursorLayer } from "../../stage/canvas/cursorLayer";
import { drawPreview } from "../../stage/canvas/previewCanvas";
import { clipMixAt } from "../../stage/clips/clipDissolve";
import { gradeCanvas } from "../../stage/grade/gradePass";
import { drawOverlays } from "../../stage/fx/overlayDraw";
import { drawMasks } from "../../stage/mask/maskDraw";
import { maskDraws } from "../../stage/mask/maskPreview";
import { layoutAt } from "../../timeline/model/layoutTrack";
import { newTilt, tiltFromCam, type TiltState } from "../../stage/cursor/cursorTilt";
import { exactKey } from "./useExactFrame";
import type { CompositeLoopRefs } from "./compositeLoopRefs";

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
      cs = r.cursorRef.current;
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
    const cm = clipMixAt(r.dissolvesRef.current, tOut, r.motionEasingRef.current);
    const b = r.screenBRef.current;
    const mix = cm && b ? { video: b, alpha: cm.alpha } : null;
    const geom = drawPreview(
      ctx,
      c.width,
      c.height,
      sv,
      r.webcamRef.current,
      cam,
      frameLayout,
      r.bgRef.current,
      t,
      s.offscreenRef.current,
      s.layerRef.current,
      tOut,
      mix,
    );
    drawMasks(
      ctx,
      c,
      maskDraws(
        r.effectsRef.current,
        frameLayout,
        cam,
        c.width,
        c.height,
        tOut,
        r.clickfxRef.current.spotlight_dim,
      ),
    );
    gradeCanvas(ctx, c, r.gradeRef.current);
    if (geom) {
      drawCursorLayer(
        ctx,
        c.width,
        c.height,
        cam,
        cur,
        r.clicksRef.current,
        t,
        r.layoutPresetsRef.current?.inset_w ?? 1,
        geom,
      );
    }
    drawOverlays(ctx, c, r, s, t, tOut, frameLayout, cam);
    const ex = r.exactRef.current;
    if (!play && ex && ex.key === exactKey(tOut, r.editGenRef.current))
      ctx.drawImage(ex.img, 0, 0, c.width, c.height);
  } catch (e) {
    if (import.meta.env.DEV) console.error("drawPreview", e);
  }
}
