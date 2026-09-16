import { fxFrameGeometry } from "./fxGeometry";
import { drawMirroredRipples, overlayNeedsClicks } from "./ripplePreview";
import { drawCaptions } from "./captionDraw";
import { resolveSpotlight, spotAlphaPlan } from "./spotlightPreview";
import type { Cam } from "../camera/camera";
import type { frameCamLayout } from "../camera/frameCam";
import { fxRequestTick } from "../../hooks/stage/fxRequestTick";
import type { FrameScratch } from "../../hooks/stage/compositeFrame";
import type { CompositeLoopRefs } from "../../hooks/stage/compositeLoopRefs";

const FX_SCALE = 0.5;

export function drawOverlays(
  ctx: CanvasRenderingContext2D,
  c: HTMLCanvasElement,
  r: CompositeLoopRefs,
  s: FrameScratch,
  t: number,
  tOut: number,
  frameLayout: ReturnType<typeof frameCamLayout>,
  cam: Cam,
): void {
  const cf = r.clickfxRef.current;
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
}
