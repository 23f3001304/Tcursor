import { useRef } from "react";
import type { CamSample, ClickSample, PreviewLayout, CursorKindSample } from "../../../shared/ipc";
import type {
  CaptionStyle,
  ClickFxSettings,
  CursorSettings,
  GradeSettings,
} from "../../../hud/settings/settings";
import type { Caption, EffectRegion, TextItem } from "../../../shared/edit";

export function useSyncRefs({
  playing,
  timeMs,
  onTime,
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
}: {
  playing: boolean;
  timeMs: number;
  onTime: (ms: number) => void;
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
}) {
  const playRef = useRef(playing);
  playRef.current = playing;
  const timeRef = useRef(timeMs);
  timeRef.current = timeMs;
  const onTimeRef = useRef(onTime);
  onTimeRef.current = onTime;
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

  return {
    playRef,
    timeRef,
    onTimeRef,
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
  };
}
