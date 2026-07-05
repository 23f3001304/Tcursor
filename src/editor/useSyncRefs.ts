import { useRef } from "react";
import type { CamSample, ClickSample, PreviewLayout, CursorKindSample } from "../lib/ipc";
import type { ClickFxSettings, CursorSettings } from "../hud/settings";
import type { EffectRegion } from "../lib/edit";

export function useSyncRefs({
  playing,
  timeMs,
  onTime,
  track,
  layout,
  clicks,
  effects,
  clickfx,
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
  cursorKinds: CursorKindSample[];
  cursor: CursorSettings;
}) {
  const playRef = useRef(playing); playRef.current = playing;
  const timeRef = useRef(timeMs); timeRef.current = timeMs;
  const onTimeRef = useRef(onTime); onTimeRef.current = onTime;
  const trackRef = useRef(track); trackRef.current = track;
  const layoutRef = useRef(layout); layoutRef.current = layout;
  const clicksRef = useRef(clicks); clicksRef.current = clicks;
  const effectsRef = useRef(effects); effectsRef.current = effects;
  const clickfxRef = useRef(clickfx); clickfxRef.current = clickfx;
  const kindsRef = useRef(cursorKinds); kindsRef.current = cursorKinds;
  const cursorRef = useRef(cursor); cursorRef.current = cursor;

  return {
    playRef,
    timeRef,
    onTimeRef,
    trackRef,
    layoutRef,
    clicksRef,
    effectsRef,
    clickfxRef,
    kindsRef,
    cursorRef,
  };
}
