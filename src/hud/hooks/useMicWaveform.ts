import { useEffect, useRef, useState } from "react";

// 13, not a round number: matches the recording bar's compact meter width (hud.css `.wave`,
// gate-feedback item 2, user-reported 2026-09-02) - 13 bars at a fixed 3px wide + 2px gap is the
// closest fit to the redesign's ~64px target (13*3 + 12*2 = 63px; 14 would overshoot to 68px).
const BARS = 13;

/** Live mic levels (0..1 per bar) from the default input, active only while `on`. Cancellable the
 *  same way `useWebcamPreview` is (`src/hud/hooks/useWebcamPreview.ts`): `on` can flip back to
 *  false, or the component can unmount, while `getUserMedia` is still pending - without the
 *  `cancelled` guard below, that stream would still open once the promise resolved (the cleanup
 *  that already ran had nothing to stop yet) and its rAF meter loop would run forever, leaking
 *  the mic indefinitely.
 *
 *  `active` (separate from `on`) is `true` only once a real stream is actually open - the caller
 *  (`Hud.tsx`) gates the recording bar's live wave on `on && active`, not `on` alone, so a
 *  permission-denied/no-device failure never renders bars that imply capture is happening (state
 *  honesty: H2/task-6 (i) - the mic icon and wave must never lie about a live device). */
export function useMicWaveform(on: boolean) {
  const [levels, setLevels] = useState<number[]>(() => Array(BARS).fill(0));
  const [active, setActive] = useState(false);
  const frame = useRef(0);
  useEffect(() => {
    if (!on) { setLevels(Array(BARS).fill(0)); setActive(false); return; }
    let cancelled = false;
    let raf = 0;
    let ctx: AudioContext | null = null;
    let stream: MediaStream | null = null;
    navigator.mediaDevices
      ?.getUserMedia({ audio: true })
      .then((s) => {
        if (cancelled) { s.getTracks().forEach((t) => t.stop()); return; }
        stream = s;
        setActive(true);
        // A track ending (device unplugged, driver reset, another app taking exclusive access)
        // is the only way the browser tells us the mic died mid-recording - drop `active` AND
        // flatten `levels` right away (fix round 1, item 3) so the wave visibly goes flat instead
        // of freezing on its last live-looking amplitude while only the icon/dim state changes.
        s.getTracks().forEach((t) => {
          t.onended = () => { if (!cancelled) { setActive(false); setLevels(Array(BARS).fill(0)); } };
        });
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
      .catch(() => setActive(false));
    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      stream?.getTracks().forEach((t) => t.stop());
      ctx?.close();
    };
  }, [on]);
  return { levels, active };
}
