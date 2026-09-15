# src/editor/util/opQueue.ts

A serial async queue factory. Plain function, no React - the ordering guarantee is unit-testable without mounting anything (`opQueue.test.ts`).

## createQueue

```ts
export function createQueue(): <T>(fn: () => Promise<T>) => Promise<T>
```

Returns an `enqueue` function backed by a private promise chain closed over in the factory call.

### Returns

`enqueue<T>(fn: () => Promise<T>): Promise<T>` - appends `fn` to the chain and returns a promise that settles with `fn`'s own result (or rejection). A call made while an earlier enqueued call is still in flight does not START running until that earlier call has settled (resolved OR rejected) - `chain.then(fn, fn)` uses `fn` as both the fulfillment and rejection handler, so one rejected step does not break the chain for subsequent `enqueue` calls.

### Behavior

- **Strict call order, not resolution order.** If three calls are enqueued with different internal delays, they still run one-at-a-time in the order they were CALLED, each waiting for the previous to settle - not in whatever order their own delays would resolve if run concurrently.
- **Independent per queue.** Each `createQueue()` call returns its own `enqueue` closed over its own chain; queues never share state.

### Notes

- **Reading fresh state at execution time.** A queued fn that reads a mutable ref (rather than closing over a captured value) sees whatever an EARLIER queued fn already wrote to that ref, even though the second fn was enqueued while the first was still in flight - `opQueue.test.ts`'s last case proves this directly with a plain `{ current }` stand-in. `Editor.tsx`'s `docRef` relies on exactly this: `applyOp`/`onRun` read `docRef.current` inside their queued closures instead of a captured `doc`, so a call queued behind an in-flight mutation observes that mutation's result, not a stale pre-mutation snapshot.

### Used by

`Editor` (`src/editor/Editor.tsx`) - one queue instance (`useRef(createQueue()).current`) shared by `applyOp`, the AI director's whole `onRun` pass, and, via the `enqueue` parameter threaded into it, `useEditHistory`'s `undo`/`redo` - so an undo/redo issued while any of those is still in flight runs only after it settles, instead of racing its `setDoc`.
