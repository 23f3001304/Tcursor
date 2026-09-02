import { useEffect, useRef } from "react";

export interface Coalesced<T> {
  (value: T): void;
  /** Apply a still-pending value immediately, with no frame delay. */
  flush(): void;
  /** Drop a still-pending value without applying it. */
  cancel(): void;
}

/** Frame-coalescing analog of `debounce` (./debounce.ts): collapses a burst of calls into at most
 *  one `onCommit`, scheduled via `schedule`/`cancelSchedule` rather than a fixed delay. Those two
 *  are injected (normally `requestAnimationFrame`/`cancelAnimationFrame`) so this stays DOM-free
 *  and unit-testable without a real animation frame - `useRafCoalesced` below is the thin React
 *  wrapper that supplies the real ones. Plain function, no React. */
export function coalesce<T>(
  onCommit: (v: T) => void,
  schedule: (cb: () => void) => number,
  cancelSchedule: (id: number) => void,
): Coalesced<T> {
  let id: number | null = null;
  let pending: { v: T } | null = null;
  const run = () => {
    if (!pending) return;
    const { v } = pending;
    pending = null;
    onCommit(v);
  };
  const coalesced = ((v: T) => {
    pending = { v };
    if (id !== null) return; // a frame is already queued - it'll pick up this latest value
    id = schedule(() => { id = null; run(); });
  }) as Coalesced<T>;
  coalesced.flush = () => { if (id !== null) { cancelSchedule(id); id = null; } run(); };
  coalesced.cancel = () => { if (id !== null) { cancelSchedule(id); id = null; } pending = null; };
  return coalesced;
}

/** Coalesces a high-frequency stream of updates (e.g. scrub pointermoves, which can fire far
 *  faster than the display refresh rate on a high-poll-rate mouse/trackpad) down to at most one
 *  `onCommit` call per animation frame, always with the LATEST value. `onCommit` is read from a
 *  ref, so callers can pass a fresh inline arrow every render without the returned `schedule`/
 *  `flush` themselves needing to be recreated. */
export function useRafCoalesced<T>(onCommit: (v: T) => void) {
  const onCommitRef = useRef(onCommit); onCommitRef.current = onCommit;
  const coalescedRef = useRef<Coalesced<T> | null>(null);
  if (!coalescedRef.current) {
    coalescedRef.current = coalesce((v: T) => onCommitRef.current(v), requestAnimationFrame, cancelAnimationFrame);
  }
  useEffect(() => () => coalescedRef.current?.cancel(), []);
  return { schedule: coalescedRef.current, flush: coalescedRef.current.flush };
}
