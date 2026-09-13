import { useEffect, useRef, type RefObject } from "react";
import type { EffectRegion } from "../../lib/edit";
import type { CamPose } from "./cameraMoves";
import { isNaturalPlaybackTick } from "./playbackTick";
import { newSpotlightSimState, spotlightEffectsKey, type SpotlightSimState } from "./spotlightPreview";
import { isGif, loopMs, type StageBg, type StageBgState } from "./stageBg";
import { closeGif, decodeGif } from "./gifFrames";

/** Everything that decides WHEN the stage must recomposite and when a live drag draft dies -
 *  extracted out of `Stage.tsx` purely to stay under its line budget (same reason
 *  `useReticleDrag` was). Four effects, each with its own reason to exist:
 *
 *  1. The background state (`StageBgState`): the export background (a data URL) decoded once per
 *     change into the `<img>` the canvas draws, and - when the doc's background is a video or GIF
 *     asset - the detached `<video>` or the decoded GIF frames that go with it. Marks the frame
 *     dirty when anything lands.
 *  2. Any draw-affecting prop change marks the canvas dirty, so a PAUSED rAF loop recomposites
 *     exactly once per change instead of redrawing a static frame at 60fps.
 *  3. Gate 2 for "spotlight freezes mid-fade": a retime/add/remove of a Spotlight region WHILE
 *     PAUSED never moves `timeMs`, so the loop's own discontinuous-jump reset (gate 1) can't fire.
 *     Keyed on spotlight CONTENT, not the `effects` array reference - `applyEditOp` hands back a
 *     fresh array on every edit routed through it, spotlight or not.
 *  4. Moving the playhead discards the unsaved Move-mode camera draft - UNLESS the `timeMs` change
 *     is just the loop's own natural playback progress rather than a real seek/scrub (M6). The
 *     draft must survive an ordinary tick whether the drag is still in progress or already
 *     released: "nothing is saved until you press the button" only holds if the draft lives that
 *     long. `playRef` keeps this reading the CURRENT `playing`, not whatever it was last run. */
export function useStageInvalidation({ bg, bgRef, dirtyRef, drawDeps, effects, spotSimRef, timeMs, playRef, camDraftRef }: {
  bg: StageBg;
  bgRef: RefObject<StageBgState | null>;
  dirtyRef: RefObject<boolean>;
  /** Every draw-affecting prop, in a stable order - the dependency list of effect 2. */
  drawDeps: unknown[];
  effects: EffectRegion[];
  spotSimRef: RefObject<SpotlightSimState>;
  timeMs: number;
  playRef: RefObject<boolean>;
  camDraftRef: RefObject<CamPose | null>;
}) {
  // The still background: one decode per change of the data URL.
  useEffect(() => {
    const st = (bgRef.current ??= { bg, img: null, video: null, gif: null, playing: false });
    st.bg = bg;
    if (!bg.url) { st.img = null; dirtyRef.current = true; return; }
    const img = new Image();
    img.onload = () => { dirtyRef.current = true; };
    img.src = bg.url;
    st.img = img;
  }, [bg.url]); // eslint-disable-line react-hooks/exhaustive-deps

  // The MOVING background, if this doc has one. A GIF cannot play in a `<video>`, so it is decoded
  // to frames instead (`gifFrames.ts`); everything else gets a detached, muted, looping element -
  // never mounted, because `drawImage` needs no DOM node. Both are torn down when the asset (or
  // the kind) changes, so switching back to a wallpaper stops the decode rather than hiding it.
  useEffect(() => {
    const st = (bgRef.current ??= { bg, img: null, video: null, gif: null, playing: false });
    st.bg = bg;
    let live = true;
    if (bg.assetUrl && isGif(bg.assetPath)) {
      decodeGif(bg.assetUrl).then((g) => {
        if (!live) { closeGif(g); return; }
        st.gif = g; dirtyRef.current = true;
      });
    } else if (bg.assetUrl) {
      const v = document.createElement("video");
      v.muted = true; v.loop = true; v.playsInline = true; v.preload = "auto"; v.src = bg.assetUrl;
      v.onloadeddata = () => { dirtyRef.current = true; };
      st.video = v;
    }
    return () => {
      live = false;
      if (st.video) { st.video.pause(); st.video.removeAttribute("src"); st.video.load(); st.video = null; }
      closeGif(st.gif); st.gif = null;
      dirtyRef.current = true;
    };
  }, [bg.assetUrl, bg.assetPath]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => { dirtyRef.current = true; }, drawDeps); // eslint-disable-line react-hooks/exhaustive-deps

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
    // A moving background follows the transport: it plays with it and seeks when paused. `playing`
    // is latched here (the draw loop reads a ref and never re-renders), and the seek-while-paused
    // is what makes scrubbing show the right frame instead of a stopped one.
    const st = bgRef.current;
    if (st?.video) {
      st.playing = playRef.current;
      if (playRef.current) { void st.video.play().catch(() => {}); }
      else { st.video.pause(); st.video.currentTime = loopMs(timeMs, (st.video.duration || 0) * 1000) / 1000; }
    }
  }, [timeMs]); // eslint-disable-line react-hooks/exhaustive-deps
}
