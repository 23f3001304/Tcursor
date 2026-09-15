import { useEffect, useRef } from "react";

export interface Coalesced<T> {
  (value: T): void;
  flush(): void;
  cancel(): void;
}

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
    if (id !== null) return;
    id = schedule(() => {
      id = null;
      run();
    });
  }) as Coalesced<T>;
  coalesced.flush = () => {
    if (id !== null) {
      cancelSchedule(id);
      id = null;
    }
    run();
  };
  coalesced.cancel = () => {
    if (id !== null) {
      cancelSchedule(id);
      id = null;
    }
    pending = null;
  };
  return coalesced;
}

export function useRafCoalesced<T>(onCommit: (v: T) => void) {
  const onCommitRef = useRef(onCommit);
  onCommitRef.current = onCommit;
  const coalescedRef = useRef<Coalesced<T> | null>(null);
  if (!coalescedRef.current) {
    coalescedRef.current = coalesce(
      (v: T) => onCommitRef.current(v),
      requestAnimationFrame,
      cancelAnimationFrame,
    );
  }
  useEffect(() => () => coalescedRef.current?.cancel(), []);
  return { schedule: coalescedRef.current, flush: coalescedRef.current.flush };
}
