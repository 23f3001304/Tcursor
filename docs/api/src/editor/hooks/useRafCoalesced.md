# src/editor/hooks/useRafCoalesced.ts

Frame-coalescing analog of `debounce.ts` - collapses a burst of calls into at most one commit,
scheduled per animation frame instead of after a fixed delay.

## coalesce

```ts
export function coalesce<T>(
  onCommit: (v: T) => void,
  schedule: (cb: () => void) => number,
  cancelSchedule: (id: number) => void,
): Coalesced<T>
```

Plain function, no React or DOM - `schedule`/`cancelSchedule` are injected (normally
`requestAnimationFrame`/`cancelAnimationFrame`) so this is unit-testable with a fake scheduler
instead of a real animation frame (`useRafCoalesced.test.ts`).

### Returns

`Coalesced<T>` - callable exactly like `(v: T) => void`, plus:

- `flush()` - applies a still-pending value immediately, cancelling the queued frame. Call this on
  pointer release so the exact release value always lands, not whatever the next scheduled frame
  happens to be.
- `cancel()` - drops a pending value without applying it.

A call while a frame is already queued does NOT schedule a second one - it just overwrites the
pending value, so the queued frame picks up the latest one when it runs.

## useRafCoalesced

```ts
export function useRafCoalesced<T>(onCommit: (v: T) => void): { schedule: Coalesced<T>; flush: () => void }
```

The React wrapper: supplies the real `requestAnimationFrame`/`cancelAnimationFrame`, reads
`onCommit` from a ref (so callers can pass a fresh inline arrow every render without `schedule`/
`flush` themselves needing to be recreated), and cancels any pending frame on unmount.

### Used by

`Timeline` (`src/editor/timeline/Timeline.tsx`) - coalesces scrub `pointermove`s (which can fire
faster than the display refresh rate on a high-poll-rate mouse/trackpad) down to at most one
`seekAt` call per animation frame, flushing on `pointerup`/`pointerLostCapture` so the release
position lands instantly.
