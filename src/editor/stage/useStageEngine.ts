import { useMemo, useRef, type RefObject } from "react";
import { newSpotlightSimState, type SpotlightSimState } from "./fx/spotlightPreview";
import { useStageInvalidation } from "./useStageInvalidation";
import type { StageBgState } from "./canvas/stageBg";
import { useArrangeDrag } from "./arrange/useArrangeDrag";
import { useStageMask } from "./mask/useStageMask";
import { useCompositeLoop } from "../hooks/stage/useCompositeLoop";
import { useExactFrame } from "../hooks/stage/useExactFrame";
import { useCursorSprites } from "../hooks/stage/useCursorSprites";
import { useMediaPlayback } from "../hooks/stage/useMediaPlayback";
import { useSyncRefs } from "../hooks/stage/useSyncRefs";
import { clipDissolves, preseekAt } from "./clips/clipDissolve";
import { outOf } from "../../shared/math/remap";
import { stageCursor } from "./stageCursor";
import type { StageProps } from "./stageProps";

export interface StageElements {
  screen: RefObject<HTMLVideoElement | null>;
  screenB: RefObject<HTMLVideoElement | null>;
  webcam: RefObject<HTMLVideoElement | null>;
  audio: RefObject<HTMLAudioElement | null>;
  canvas: RefObject<HTMLCanvasElement | null>;
}

export function useStageEngine(p: StageProps, el: StageElements, canvasW: number, canvasH: number) {
  const bgImg = useRef<StageBgState | null>(null);
  const dirtyRef = useRef(true);
  const { captured, plainOs, effCursor } = stageCursor(p.cursor, p.osCursorInVideo, p.cursorLayer);
  const tOut = outOf(p.map, p.timeMs);
  const arrange = useArrangeDrag({
    seg: p.arrangeSeg,
    presets: p.layoutPresets,
    canvasRef: el.canvas,
    canvasW,
    canvasH,
    dirtyRef,
    onApply: p.onApply,
  });
  const arranging = p.arrangeSeg !== null;
  const mask = useStageMask(p, canvasW, canvasH, tOut, dirtyRef, arranging);
  const dissolves = useMemo(() => clipDissolves(p.map, p.clips), [p.map, p.clips]);
  const preseekMs = useMemo(() => preseekAt(dissolves, p.map, tOut), [dissolves, p.map, tOut]);
  const refs = useSyncRefs({
    playing: p.playing,
    timeMs: p.timeMs,
    onTime: p.onTime,
    onSeek: p.onSeek,
    track: p.track,
    layout: p.layout,
    clicks: p.clicks,
    effects: p.effects,
    clickfx: p.clickfx,
    grade: p.grade,
    captions: p.captions,
    texts: p.texts,
    capStyle: p.capStyle,
    accent: p.accent,
    cursorKinds: plainOs ? [] : p.cursorKinds,
    cursor: effCursor,
    layoutPresets: arrange.presets,
    layoutSegs: p.layoutSegs,
    cameraMoves: p.cameraMoves,
    zooms: p.zooms,
    zoomSettings: p.zoomSettings,
    arranging,
    map: p.map,
    dissolves,
    motionEasing: p.motionEasing,
  });
  const trailRef = useRef<[number, number][]>([]);
  const spritesRef = useCursorSprites(p.cursorSprites, captured, () => {
    dirtyRef.current = true;
  });
  const spotSimRef = useRef<SpotlightSimState>(newSpotlightSimState());

  // INVARIANT: both dep lists below must keep a constant length across renders.
  const sceneDeps = [
    p.track,
    p.layout,
    arrange.presets,
    p.layoutSegs,
    p.cameraMoves,
    p.zooms,
    p.zoomSettings,
    p.clicks,
    p.effects,
    p.cursor,
    p.clickfx,
    p.grade,
    p.cursorKinds,
    captured,
    arranging,
    p.captions,
    p.capStyle,
    p.accent,
    p.texts,
  ];

  useStageInvalidation({
    bg: p.bg,
    bgRef: bgImg,
    dirtyRef,
    effects: p.effects,
    spotSimRef,
    timeMs: tOut,
    playRef: refs.playRef,
    camDraftRef: p.camDraftRef,
    drawDeps: [p.timeMs, p.playing, ...sceneDeps],
  });

  useMediaPlayback({
    screenRef: el.screen,
    screenBRef: el.screenB,
    webcamRef: el.webcam,
    audioRef: el.audio,
    playing: p.playing,
    src: p.src,
    muted: p.muted,
    volume: p.volume,
    audioSrc: p.audioSrc,
    timeMs: p.timeMs,
    preseekMs,
    playRef: refs.playRef,
  });
  const exact = useExactFrame({
    folder: p.folder,
    playing: p.playing,
    outMs: tOut,
    draft: p.moveMode || arranging,
    dirtyRef,
    deps: [...sceneDeps, p.moveMode, p.bg],
  });

  useCompositeLoop({
    ...refs,
    screenRef: el.screen,
    screenBRef: el.screenB,
    webcamRef: el.webcam,
    audioRef: el.audio,
    canvasRef: el.canvas,
    dragPoseRef: p.camDraftRef,
    spritesRef,
    trailRef,
    dirtyRef,
    bgRef: bgImg,
    spotSimRef,
    exactRef: exact.exactRef,
    editGenRef: exact.editGenRef,
  });

  return {
    arrange,
    arranging,
    mask,
    dirtyRef,
    tOut,
    wantsScreenB: dissolves.length > 0,
    mapRef: refs.mapRef,
    layoutRef: refs.layoutRef,
    trackRef: refs.trackRef,
    timeRef: refs.timeRef,
    playRef: refs.playRef,
    onTimeRef: refs.onTimeRef,
  };
}
