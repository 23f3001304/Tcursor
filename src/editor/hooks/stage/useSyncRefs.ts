import { useRef } from "react";
import type { CamSample, ClickSample, PreviewLayout, CursorKindSample } from "../../../shared/ipc";
import type { CaptionStyle, ClickFxSettings, CursorSettings } from "../../../hud/settings/settings";
import type { Caption, EffectRegion } from "../../../shared/edit";

export function useSyncRefs({
  playing,
  timeMs,
  onTime,
  track,
  layout,
  clicks,
  effects,
  clickfx,
  captions,
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
  captions: Caption[];
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
  const captionsRef = useRef(captions);
  captionsRef.current = captions;
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
    captionsRef,
    capStyleRef,
    accentRef,
    kindsRef,
    cursorRef,
  };
}
