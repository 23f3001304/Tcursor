import { useEffect, useRef, type RefObject } from "react";

export function useMediaPlayback({
  screenRef,
  webcamRef,
  audioRef,
  playing,
  src,
  muted,
  volume,
  audioSrc,
  timeMs,
  playRef,
}: {
  screenRef: RefObject<HTMLVideoElement | null>;
  webcamRef: RefObject<HTMLVideoElement | null>;
  audioRef: RefObject<HTMLAudioElement | null>;
  playing: boolean;
  src: string;
  muted: boolean;
  volume: number;
  audioSrc: string;
  timeMs: number;
  playRef: RefObject<boolean>;
}) {
  const timeRef = useRef(timeMs);
  timeRef.current = timeMs;

  useEffect(() => {
    const sv = screenRef.current;
    const wv = webcamRef.current;
    const av = audioRef.current;
    if (playing) {
      const t = Math.max(0, timeRef.current / 1000);
      for (const m of [sv, wv, av]) {
        if (m && Math.abs(m.currentTime - t) > 0.05) m.currentTime = t;
      }
      sv?.play().catch(() => {});
      wv?.play().catch(() => {});
      av?.play().catch(() => {});
    } else {
      sv?.pause();
      wv?.pause();
      av?.pause();
    }
  }, [playing, src, screenRef, webcamRef, audioRef]);

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.muted = muted;
      audioRef.current.volume = Math.max(0, Math.min(1, volume));
    }
  }, [muted, volume, audioSrc, audioRef]);

  useEffect(() => {
    if (playRef.current) return;
    const sv = screenRef.current;
    const wv = webcamRef.current;
    const av = audioRef.current;
    if (sv && Math.abs(sv.currentTime * 1000 - timeMs) > 40) {
      sv.currentTime = Math.max(0, timeMs / 1000);
    }
    if (wv && Math.abs(wv.currentTime * 1000 - timeMs) > 40) {
      wv.currentTime = Math.max(0, timeMs / 1000);
    }
    if (av && Math.abs(av.currentTime * 1000 - timeMs) > 40) {
      av.currentTime = Math.max(0, timeMs / 1000);
    }
  }, [timeMs, playing, screenRef, webcamRef, audioRef, playRef]);
}
