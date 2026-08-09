/** A serial async queue: each `enqueue(fn)` call appends `fn` to a private promise chain, so a
 *  call that arrives while an earlier one is still in flight does not start until that earlier
 *  one has SETTLED (resolved or rejected) - regardless of how long each one takes on its own.
 *  Plain function factory (no React) so the ordering guarantee is unit-testable without mounting
 *  anything. `Editor` uses this to serialize every `EditDoc` mutation (`applyOp`, `undo`, `redo`)
 *  through one queue, so e.g. an undo issued while an `applyOp` is still awaiting its IPC round
 *  trip runs AFTER that apply resolves, reverting the doc it actually settled to - not a stale
 *  one the in-flight apply is about to overwrite. */
export function createQueue() {
  let chain: Promise<unknown> = Promise.resolve();
  return function enqueue<T>(fn: () => Promise<T>): Promise<T> {
    const next = chain.then(fn, fn) as Promise<T>;
    chain = next;
    return next;
  };
}
