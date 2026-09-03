import { useEffect, useRef, type RefObject } from "react";
import type { EffectRegion } from "../../lib/edit";
import type { CamPose } from "./cameraMoves";
import { isNaturalPlaybackTick } from "./playbackTick";
import { newSpotlightSimState, spotlightEffectsKey, type SpotlightSimState } from "./spotlightPreview";

/** Everything that decides WHEN the stage must recomposite and when a live drag draft dies -
 *  extracted out of `Stage.tsx` purely to stay under its line budget (same reason
 *  `useReticleDrag` was). Four effects, each with its own reason to exist:
 *
 *  1. The export background (a data URL) is decoded once per change into the `<img>` the canvas
 *     draws, marking the frame dirty when it lands.
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
export function useStageInvalidation({ bgUrl, bgImgRef, dirtyRef, drawDeps, effects, spotSimRef, timeMs, playRef, camDraftRef }: {
  bgUrl: string;
  bgImgRef: RefObject<HTMLImageElement | null>;
  dirtyRef: RefObject<boolean>;
  /** Every draw-affecting prop, in a stable order - the dependency list of effect 2. */
  drawDeps: unknown[];
  effects: EffectRegion[];
  spotSimRef: RefObject<SpotlightSimState>;
  timeMs: number;
  playRef: RefObject<boolean>;
  camDraftRef: RefObject<CamPose | null>;
}) {
  useEffect(() => {
    if (!bgUrl) { bgImgRef.current = null; dirtyRef.current = true; return; }
    const img = new Image();
    img.onload = () => { dirtyRef.current = true; };
    img.src = bgUrl;
    bgImgRef.current = img;
  }, [bgUrl]); // eslint-disable-line react-hooks/exhaustive-deps

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
  }, [timeMs]); // eslint-disable-line react-hooks/exhaustive-deps
}
