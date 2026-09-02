export interface Debounced<A extends unknown[]> {
  (...args: A): void;
  /** Drop any pending trailing call without running it. */
  cancel(): void;
  /** Run a pending trailing call immediately (no-op if nothing is pending). */
  flush(): void;
}

/** Trailing-edge debounce: `fn` runs `wait` ms after the LAST call, with only the latest args -
 *  a burst of calls inside one `wait` window collapses to one. `cancel()` drops a pending call
 *  (used on unmount, so a stale caller can't fire after its owner is gone); `flush()` runs a
 *  pending call right away (used on pointer release, so a drag's final value commits instantly
 *  instead of waiting out the trailing window). Plain function, no React - unit-testable with
 *  fake timers alone. */
export function debounce<A extends unknown[]>(fn: (...args: A) => void, wait: number): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let pending: A | null = null;

  const run = () => {
    if (!pending) return;
    const args = pending;
    pending = null;
    fn(...args);
  };

  const debounced = ((...args: A) => {
    pending = args;
    if (timer !== null) clearTimeout(timer);
    timer = setTimeout(() => { timer = null; run(); }, wait);
  }) as Debounced<A>;

  debounced.cancel = () => {
    if (timer !== null) { clearTimeout(timer); timer = null; }
    pending = null;
  };
  debounced.flush = () => {
    if (timer !== null) { clearTimeout(timer); timer = null; }
    run();
  };

  return debounced;
}
