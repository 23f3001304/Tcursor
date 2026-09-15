import { useRef, type RefObject } from "react";
import { newSpotlightSimState, type SpotlightSimState } from "./fx/spotlightPreview";
import { useStageInvalidation } from "./useStageInvalidation";
import type { StageBgState } from "./canvas/stageBg";
import { useArrangeDrag } from "./arrange/useArrangeDrag";
import { useCompositeLoop } from "../hooks/stage/useCompositeLoop";
import { useExactFrame } from "../hooks/stage/useExactFrame";
import { useCursorSprites } from "../hooks/stage/useCursorSprites";
import { useMediaPlayback } from "../hooks/stage/useMediaPlayback";
import { useSyncRefs } from "../hooks/stage/useSyncRefs";
import { outOf } from "../../shared/math/remap";
import { stageCursor } from "./stageCursor";
import type { StageProps } from "./stageProps";

export interface StageElements {
  screen: RefObject<HTMLVideoElement | null>;
  webcam: RefObject<HTMLVideoElement | null>;
  audio: RefObject<HTMLAudioElement | null>;
  canvas: RefObject<HTMLCanvasElement | null>;
}

export function useStageEngine(p: StageProps, el: StageElements, canvasW: number, canvasH: number) {
  const bgImg = useRef<StageBgState | null>(null);
  const dirtyRef = useRef(true);
  const { captured, plainOs, effCursor } = stageCursor(p.cursor, p.osCursorInVideo, p.cursorLayer);
  const tOut = outOf(p.map, p.timeMs);
  const mapRef = useRef(p.map);
  mapRef.current = p.map;
  const {
    playRef,
    timeRef,
    onTimeRef,
    trackRef,
    layoutRef,
    clicksRef,
    effectsRef,
    clickfxRef,
    captionsRef,
    capStyleRef,
    accentRef,
    kindsRef,
    cursorRef,
  } = useSyncRefs({
    playing: p.playing,
    timeMs: p.timeMs,
    onTime: p.onTime,
    track: p.track,
    layout: p.layout,
    clicks: p.clicks,
    effects: p.effects,
    clickfx: p.clickfx,
    captions: p.captions,
    capStyle: p.capStyle,
    accent: p.accent,
    cursorKinds: plainOs ? [] : p.cursorKinds,
    cursor: effCursor,
  });
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
  const arrangingRef = useRef(arranging);
  arrangingRef.current = arranging;
  const layoutPresetsRef = useRef(arrange.presets);
  layoutPresetsRef.current = arrange.presets;
  const layoutSegsRef = useRef(p.layoutSegs);
  layoutSegsRef.current = p.layoutSegs;
  const cameraMovesRef = useRef(p.cameraMoves);
  cameraMovesRef.current = p.cameraMoves;
  const zoomsRef = useRef(p.zooms);
  zoomsRef.current = p.zooms;
  const zoomSettingsRef = useRef(p.zoomSettings);
  zoomSettingsRef.current = p.zoomSettings;
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
    p.cursorKinds,
    captured,
    arranging,
    p.captions,
    p.capStyle,
    p.accent,
  ];

  useStageInvalidation({
    bg: p.bg,
    bgRef: bgImg,
    dirtyRef,
    effects: p.effects,
    spotSimRef,
    timeMs: tOut,
    playRef,
    camDraftRef: p.camDraftRef,
    drawDeps: [p.timeMs, p.playing, ...sceneDeps],
  });

  useMediaPlayback({
    screenRef: el.screen,
    webcamRef: el.webcam,
    audioRef: el.audio,
    playing: p.playing,
    src: p.src,
    muted: p.muted,
    volume: p.volume,
    audioSrc: p.audioSrc,
    timeMs: p.timeMs,
    playRef,
  });
  const exact = useExactFrame({
    folder: p.folder,
    playing: p.playing,
    timeMs: p.timeMs,
    draft: p.moveMode || arranging,
    dirtyRef,
    deps: [...sceneDeps, p.moveMode, p.bg],
  });

  useCompositeLoop({
    screenRef: el.screen,
    webcamRef: el.webcam,
    audioRef: el.audio,
    canvasRef: el.canvas,
    playRef,
    timeRef,
    onTimeRef,
    trackRef,
    layoutRef,
    layoutPresetsRef,
    layoutSegsRef,
    cameraMovesRef,
    zoomsRef,
    zoomSettingsRef,
    dragPoseRef: p.camDraftRef,
    arrangingRef,
    clicksRef,
    effectsRef,
    clickfxRef,
    kindsRef,
    cursorRef,
    captionsRef,
    capStyleRef,
    accentRef,
    spritesRef,
    trailRef,
    dirtyRef,
    bgRef: bgImg,
    spotSimRef,
    mapRef,
    exactRef: exact.exactRef,
    editGenRef: exact.editGenRef,
  });

  return { arrange, arranging, dirtyRef, tOut, mapRef, layoutRef, trackRef, timeRef, playRef, onTimeRef };
}
