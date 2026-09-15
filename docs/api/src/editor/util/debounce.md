# src/editor/util/debounce.ts

A trailing-edge debounce factory. Plain function, no React - unit-testable with fake timers
alone (`debounce.test.ts`).

## debounce

```ts
export function debounce<A extends unknown[]>(fn: (...args: A) => void, wait: number): Debounced<A>
```

Returns a wrapped `fn` that only actually runs `wait` ms after the LAST call - a burst of calls
inside one `wait` window collapses to a single run, with the latest call's args.

### Returns

`Debounced<A>` - callable exactly like `fn`, plus:

- `cancel()` - drops a pending call without running it. Call this on unmount so a caller that
  scheduled a trailing call can't fire after its owner is gone.
- `flush()` - runs a pending call right away, synchronously, instead of waiting out the trailing
  window. Call this on pointer release (or any other "commit now" moment) so a drag's final value
  reaches its destination instantly rather than lagging by up to `wait` ms.

### Used by

- `Slider` (`src/editor/controls/fields/Slider.tsx`) - wraps the caller's `onChange` so a pointer drag
  reports its optimistic value on every move (local UI, no debounce) but only actually CALLS
  `onChange` (which typically fires an `apply_edit_op`/`save_edit` IPC round trip) at most every
  80ms while dragging, `flush()`ed on release so the final value always lands.
- `useEditorData` (`src/editor/hooks/doc/useEditorData.ts`) - debounces the `preview_bg` refetch
  (keyed on the doc's background settings) so a burst of background-settings writes doesn't
  re-encode the preview background once per write.
- `Stage` (`src/editor/stage/Stage.tsx`) - wraps the on-stage reticle drag's `onAimAt` calls the
  same way `Slider` wraps `onChange`.
