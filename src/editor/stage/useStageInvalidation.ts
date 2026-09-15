import { useEffect, useRef, type RefObject } from "react";
import type { EffectRegion } from "../../shared/edit";
import type { CamPose } from "./camera/cameraMoves";
import { isNaturalPlaybackTick } from "./transport/playback";
import { newSpotlightSimState, spotlightEffectsKey, type SpotlightSimState } from "./fx/spotlightPreview";
import { isGif, loopMs, type StageBg, type StageBgState } from "./canvas/stageBg";
import { closeGif, decodeGif } from "./canvas/gifFrames";

export function useStageInvalidation({
  bg,
  bgRef,
  dirtyRef,
  drawDeps,
  effects,
  spotSimRef,
  timeMs,
  playRef,
  camDraftRef,
}: {
  bg: StageBg;
  bgRef: RefObject<StageBgState | null>;
  dirtyRef: RefObject<boolean>;
  drawDeps: unknown[];
  effects: EffectRegion[];
  spotSimRef: RefObject<SpotlightSimState>;
  timeMs: number;
  playRef: RefObject<boolean>;
  camDraftRef: RefObject<CamPose | null>;
}) {
  useEffect(() => {
    const st = (bgRef.current ??= { bg, img: null, video: null, gif: null, playing: false });
    st.bg = bg;
    if (!bg.url) {
      st.img = null;
      dirtyRef.current = true;
      return;
    }
    const img = new Image();
    img.onload = () => {
      dirtyRef.current = true;
    };
    img.src = bg.url;
    st.img = img;
  }, [bg.url]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    const st = (bgRef.current ??= { bg, img: null, video: null, gif: null, playing: false });
    st.bg = bg;
    let live = true;
    if (bg.assetUrl && isGif(bg.assetPath)) {
      decodeGif(bg.assetUrl).then((g) => {
        if (!live) {
          closeGif(g);
          return;
        }
        st.gif = g;
        dirtyRef.current = true;
      });
    } else if (bg.assetUrl) {
      const v = document.createElement("video");
      v.muted = true;
      v.loop = true;
      v.playsInline = true;
      v.preload = "auto";
      v.src = bg.assetUrl;
      v.onloadeddata = () => {
        dirtyRef.current = true;
      };
      st.video = v;
    }
    return () => {
      live = false;
      if (st.video) {
        st.video.pause();
        st.video.removeAttribute("src");
        st.video.load();
        st.video = null;
      }
      closeGif(st.gif);
      st.gif = null;
      dirtyRef.current = true;
    };
  }, [bg.assetUrl, bg.assetPath]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    dirtyRef.current = true;
  }, drawDeps); // eslint-disable-line react-hooks/exhaustive-deps

  const spotKeyRef = useRef(spotlightEffectsKey(effects));
  useEffect(() => {
    const key = spotlightEffectsKey(effects);
    if (key !== spotKeyRef.current) spotSimRef.current = newSpotlightSimState();
    spotKeyRef.current = key;
  }, [effects]); // eslint-disable-line react-hooks/exhaustive-deps

  const lastCamTimeRef = useRef(timeMs);
  useEffect(() => {
    dirtyRef.current = true;
    if (!isNaturalPlaybackTick(lastCamTimeRef.current, timeMs, playRef.current)) camDraftRef.current = null;
    lastCamTimeRef.current = timeMs;
    const st = bgRef.current;
    if (st?.video) {
      st.playing = playRef.current;
      if (playRef.current) {
        void st.video.play().catch(() => {});
      } else {
        st.video.pause();
        st.video.currentTime = loopMs(timeMs, (st.video.duration || 0) * 1000) / 1000;
      }
    }
  }, [timeMs]); // eslint-disable-line react-hooks/exhaustive-deps
}
