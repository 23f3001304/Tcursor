import { useEffect, useRef, useState } from "react";

const BARS = 14;

/** Live mic levels (0..1 per bar) from the default input, active only while `on`. Cancellable the
 *  same way `useWebcamPreview` is (`src/hud/hooks/useWebcamPreview.ts`): `on` can flip back to
 *  false, or the component can unmount, while `getUserMedia` is still pending - without the
 *  `cancelled` guard below, that stream would still open once the promise resolved (the cleanup
 *  that already ran had nothing to stop yet) and its rAF meter loop would run forever, leaking
 *  the mic indefinitely. */
export function useMicWaveform(on: boolean) {
  const [levels, setLevels] = useState<number[]>(() => Array(BARS).fill(0));
  const frame = useRef(0);
  useEffect(() => {
    if (!on) { setLevels(Array(BARS).fill(0)); return; }
    let cancelled = false;
    let raf = 0;
    let ctx: AudioContext | null = null;
    let stream: MediaStream | null = null;
    navigator.mediaDevices
      ?.getUserMedia({ audio: true })
      .then((s) => {
        if (cancelled) { s.getTracks().forEach((t) => t.stop()); return; }
        stream = s;
        ctx = new AudioContext();
        const analyser = ctx.createAnalyser();
        analyser.fftSize = 64;
        ctx.createMediaStreamSource(s).connect(analyser);
        const data = new Uint8Array(analyser.frequencyBinCount);
        const tick = () => {
          raf = requestAnimationFrame(tick);
          if (frame.current++ % 3 !== 0) return; // throttle to ~20fps
          analyser.getByteFrequencyData(data);
          setLevels(Array.from({ length: BARS }, (_, i) => (data[i * 2] ?? 0) / 255));
        };
        tick();
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      stream?.getTracks().forEach((t) => t.stop());
      ctx?.close();
    };
  }, [on]);
  return levels;
}
