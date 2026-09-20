import { useEffect, useRef } from "react";
import { outOf } from "../../../shared/math/remap";
import { isCutJump, playbackAction } from "../../stage/transport/playback";
import { clipTick, needsOutClock } from "../../stage/transport/outClock";
import { newSpotlightSimState } from "../../stage/fx/spotlightPreview";
import { resetTilt } from "../../stage/cursor/cursorTilt";
import type { CompositeLoopRefs } from "./compositeLoopRefs";
import { drawCompositeFrame, newFrameScratch } from "./compositeFrame";

export function useCompositeLoop(refs: CompositeLoopRefs) {
  const {
    screenRef,
    webcamRef,
    audioRef,
    canvasRef,
    playRef,
    timeRef,
    onTimeRef,
    onSeekRef,
    trailRef,
    dirtyRef,
    spotSimRef,
    mapRef,
  } = refs;
  const lastReportRef = useRef(0);
  const lastFrameTRef = useRef(0);
  const lastOutTRef = useRef(0);
  const outClockRef = useRef(0);
  const lastMapRef = useRef(mapRef.current);
  const scratch = useRef(newFrameScratch()).current;

  useEffect(() => {
    let raf = 0;
    const tick = () => {
      const sv = screenRef.current,
        c = canvasRef.current,
        play = playRef.current;
      const map = mapRef.current;
      const outMode = needsOutClock(map);
      const remapped = map !== lastMapRef.current;
      lastMapRef.current = map;
      if (!play || !outMode || remapped)
        outClockRef.current = outOf(map, play && sv ? sv.currentTime * 1000 : timeRef.current);
      const step = outMode && sv && play ? clipTick(map, outClockRef.current, sv.currentTime * 1000) : null;
      if (step) outClockRef.current = step.tOut;
      const cut = step ? step.seekTo : sv && play ? playbackAction(map, sv.currentTime * 1000).seekTo : null;
      if (cut !== null) {
        for (const m of [sv, webcamRef.current, audioRef.current]) if (m) m.currentTime = cut / 1000;
      }
      if (sv && c && cut === null && (play || dirtyRef.current)) {
        dirtyRef.current = false;
        const t = step ? step.t : play ? sv.currentTime * 1000 : timeRef.current;
        const tOut = step ? Math.round(step.tOut) : outOf(map, t);
        if (play) {
          const rate = playbackAction(map, t).rate;
          for (const m of [sv, webcamRef.current, audioRef.current])
            if (m && m.playbackRate !== rate) m.playbackRate = rate;
        }
        const dtMs = t - lastFrameTRef.current;
        const jump =
          (Math.abs(dtMs) > 200 || t < lastFrameTRef.current) && !isCutJump(map, lastFrameTRef.current, t);
        if (jump) {
          trailRef.current.length = 0;
          resetTilt(scratch.tiltRef.current);
          spotSimRef.current = newSpotlightSimState();
        }
        const dtOut = jump ? 0 : Math.max(0, tOut - lastOutTRef.current);
        lastFrameTRef.current = t;
        lastOutTRef.current = tOut;
        if (play) {
          if (step?.ended) onSeekRef.current(step.t);
          else if (t < lastReportRef.current || t - lastReportRef.current >= 60) {
            onTimeRef.current(t);
            lastReportRef.current = t;
          }
          const wv = webcamRef.current,
            av = audioRef.current;
          if (wv && Math.abs(wv.currentTime - sv.currentTime) > 0.15) wv.currentTime = sv.currentTime;
          if (av && Math.abs(av.currentTime - sv.currentTime) > 0.18) av.currentTime = sv.currentTime;
        }
        const ctx = c.getContext("2d");
        if (ctx) drawCompositeFrame(ctx, c, sv, refs, scratch, play, t, tOut, dtOut);
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
