import { useRef } from "react";
import type {
  CamSample,
  ClickSample,
  PreviewLayout,
  CursorKindSample,
  LayoutPresets,
} from "../../../shared/ipc";
import type {
  CaptionStyle,
  ClickFxSettings,
  CursorSettings,
  GradeSettings,
  ZoomSettings,
} from "../../../hud/settings/settings";
import type { CameraMove, Caption, EffectRegion, LayoutSeg, TextItem, Zoom } from "../../../shared/edit";
import type { TimeMap } from "../../../shared/math/remap";
import type { ClipDissolve } from "../../stage/clips/clipDissolve";

export function useSyncRefs({
  playing,
  timeMs,
  onTime,
  onSeek,
  track,
  layout,
  clicks,
  effects,
  clickfx,
  grade,
  captions,
  texts,
  capStyle,
  accent,
  cursorKinds,
  cursor,
  layoutPresets,
  layoutSegs,
  cameraMoves,
  zooms,
  zoomSettings,
  arranging,
  map,
  dissolves,
  motionEasing,
}: {
  playing: boolean;
  timeMs: number;
  onTime: (ms: number) => void;
  onSeek: (ms: number) => void;
  track: CamSample[];
  layout: PreviewLayout | null;
  clicks: ClickSample[];
  effects: EffectRegion[];
  clickfx: ClickFxSettings;
  grade: GradeSettings;
  captions: Caption[];
  texts: TextItem[];
  capStyle: CaptionStyle;
  accent: [number, number, number];
  cursorKinds: CursorKindSample[];
  cursor: CursorSettings;
  layoutPresets: LayoutPresets | null;
  layoutSegs: LayoutSeg[];
  cameraMoves: CameraMove[];
  zooms: Zoom[];
  zoomSettings: ZoomSettings;
  arranging: boolean;
  map: TimeMap;
  dissolves: ClipDissolve[];
  motionEasing: string;
}) {
  const playRef = useRef(playing);
  playRef.current = playing;
  const timeRef = useRef(timeMs);
  timeRef.current = timeMs;
  const onTimeRef = useRef(onTime);
  onTimeRef.current = onTime;
  const onSeekRef = useRef(onSeek);
  onSeekRef.current = onSeek;
  const trackRef = useRef(track);
  trackRef.current = track;
  const layoutRef = useRef(layout);
  layoutRef.current = layout;
  const clicksRef = useRef(clicks);
  clicksRef.current = clicks;
  const effectsRef = useRef(effects);
  effectsRef.current = effects;
  const clickfxRef = useRef(clickfx);
  clickfxRef.current = clickfx;
  const gradeRef = useRef(grade);
  gradeRef.current = grade;
  const captionsRef = useRef(captions);
  captionsRef.current = captions;
  const textsRef = useRef(texts);
  textsRef.current = texts;
  const capStyleRef = useRef(capStyle);
  capStyleRef.current = capStyle;
  const accentRef = useRef(accent);
  accentRef.current = accent;
  const kindsRef = useRef(cursorKinds);
  kindsRef.current = cursorKinds;
  const cursorRef = useRef(cursor);
  cursorRef.current = cursor;
  const layoutPresetsRef = useRef(layoutPresets);
  layoutPresetsRef.current = layoutPresets;
  const layoutSegsRef = useRef(layoutSegs);
  layoutSegsRef.current = layoutSegs;
  const cameraMovesRef = useRef(cameraMoves);
  cameraMovesRef.current = cameraMoves;
  const zoomsRef = useRef(zooms);
  zoomsRef.current = zooms;
  const zoomSettingsRef = useRef(zoomSettings);
  zoomSettingsRef.current = zoomSettings;
  const arrangingRef = useRef(arranging);
  arrangingRef.current = arranging;
  const mapRef = useRef(map);
  mapRef.current = map;
  const dissolvesRef = useRef(dissolves);
  dissolvesRef.current = dissolves;
  const motionEasingRef = useRef(motionEasing);
  motionEasingRef.current = motionEasing;

  return {
    playRef,
    timeRef,
    onTimeRef,
    onSeekRef,
    trackRef,
    layoutRef,
    clicksRef,
    effectsRef,
    clickfxRef,
    gradeRef,
    captionsRef,
    textsRef,
    capStyleRef,
    accentRef,
    kindsRef,
    cursorRef,
    layoutPresetsRef,
    layoutSegsRef,
    cameraMovesRef,
    zoomsRef,
    zoomSettingsRef,
    arrangingRef,
    mapRef,
    dissolvesRef,
    motionEasingRef,
  };
}
