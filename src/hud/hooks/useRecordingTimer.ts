import { useEffect, useRef, useState } from "react";

export function useRecordingTimer(active: boolean, paused: boolean) {
  const [elapsed, setElapsed] = useState(0);
  const acc = useRef(0);
  const segStart = useRef(0);
  useEffect(() => {
    if (!active) {
      acc.current = 0;
      setElapsed(0);
      return;
    }
    if (paused) return;
    segStart.current = Date.now();
    const id = setInterval(() => setElapsed(acc.current + (Date.now() - segStart.current)), 200);
    return () => {
      acc.current += Date.now() - segStart.current;
      clearInterval(id);
    };
  }, [active, paused]);
  return elapsed;
}
