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
  volume: number; // 0..1 preview-audio gain (the transport slider); separate from `muted`
  audioSrc: string;
  timeMs: number;
  playRef: RefObject<boolean>;
}) {
  // Latest playhead, read (not depended on) by the play effect so it can seek on the play
  // transition without re-running on every scrub tick.
  const timeRef = useRef(timeMs);
  timeRef.current = timeMs;

  useEffect(() => {
    const sv = screenRef.current;
    const wv = webcamRef.current;
    const av = audioRef.current;
    if (playing) {
      // Seek to the current time BEFORE playing. The currentTime-sync effect below is disabled
      // while playing (its `playRef.current` guard), so a seek requested in the same commit as play
      // - e.g. Play snapping a past-the-trim-out playhead back to the trim-in - would otherwise
      // never reach the media: the video would resume from where it stopped, re-trip the trim
      // clamp, and creep one more frame past the boundary on every Play.
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
