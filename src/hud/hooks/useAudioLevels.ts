import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface AudioLevel {
  source: "mic" | "system";
  rms: number;
}

const STALE_MS = 400;

export function useAudioLevels(on: boolean): { read: () => { mic: number; sys: number }; live: boolean } {
  const levels = useRef({ mic: 0, sys: 0, micAt: 0, sysAt: 0 });
  const [live, setLive] = useState(false);

  const read = useCallback(() => {
    const now = performance.now();
    const l = levels.current;
    return {
      mic: now - l.micAt < STALE_MS ? l.mic : 0,
      sys: now - l.sysAt < STALE_MS ? l.sys : 0,
    };
  }, []);

  useEffect(() => {
    levels.current = { mic: 0, sys: 0, micAt: 0, sysAt: 0 };
    if (!on) {
      setLive(false);
      return;
    }
    const un = listen<AudioLevel>("audio-level", (e) => {
      const now = performance.now();
      const l = levels.current;
      if (e.payload.source === "mic") {
        l.mic = e.payload.rms;
        l.micAt = now;
      } else {
        l.sys = e.payload.rms;
        l.sysAt = now;
      }
      setLive(true);
    });
    const stale = setInterval(() => {
      const now = performance.now();
      const l = levels.current;
      setLive(now - l.micAt < STALE_MS || now - l.sysAt < STALE_MS);
    }, 250);
    return () => {
      clearInterval(stale);
      void un.then((f) => f());
    };
  }, [on]);

  return { read, live };
}
