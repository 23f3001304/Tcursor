import { useEffect, useState } from "react";

/** Live `prefers-reduced-motion` - checked independent of any Motion `MotionConfig` ancestor (the
 *  editor tree has none; only the HUD's does), so a component honors it wherever it mounts, and
 *  reacts immediately if the user flips the OS setting without reloading.
 *
 *  Lifted out of `TcursorMark.tsx` when the wave motif grew past the brand mark: every wave in the
 *  app (the recording meter, the processing sweep, the playhead ripple, the horizon) has to answer
 *  the same question, and a second copy of this would be a second chance to get it wrong. */
export function useReducedMotion(): boolean {
  const supported = typeof matchMedia === "function";
  const [reduced, setReduced] = useState(() => supported && matchMedia("(prefers-reduced-motion: reduce)").matches);
  useEffect(() => {
    if (!supported) return;
    const mq = matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = () => setReduced(mq.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, [supported]);
  return reduced;
}
