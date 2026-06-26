import { useEffect, useRef, useState } from "react";

/** Elapsed recording time in ms, frozen while `paused`, reset when not `active`. */
export function useRecordingTimer(active: boolean, paused: boolean) {
  const [elapsed, setElapsed] = useState(0);
  const acc = useRef(0);       // ms accumulated before the current running segment
  const segStart = useRef(0);  // Date.now() when the current segment began
  useEffect(() => {
    if (!active) { acc.current = 0; setElapsed(0); return; }
    if (paused) return; // frozen: keep the accumulated value
    segStart.current = Date.now();
    const id = setInterval(
      () => setElapsed(acc.current + (Date.now() - segStart.current)),
      200,
    );
    return () => {
      acc.current += Date.now() - segStart.current;
      clearInterval(id);
    };
  }, [active, paused]);
  return elapsed;
}
