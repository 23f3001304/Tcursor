# src/editor/util/overrideClear.ts

A pure predicate shared by `Slider.tsx` and `Stage.tsx` for clearing a gesture-scoped optimistic
override. Plain function, no React - unit-testable directly (`overrideClear.test.ts`).

## shouldClearOverride

```ts
export function shouldClearOverride<T>(
  gestureActive: boolean,
  overrideSet: boolean,
  current: T,
  settled: T,
  eq?: (a: T, b: T) => boolean, // default Object.is
): boolean
```

Whether a gesture-scoped optimistic override (`Slider`'s `dragValue`, `Stage`'s reticle-drag
`liveAim`) should clear.

### Behavior

Returns `true` only when `!gestureActive && overrideSet && !eq(current, settled)`.

Clears on the FIRST change away from `settled` (the source-of-truth value as of the moment the
gesture began) - whatever it changes TO - rather than requiring the source value to land back on
the exact value the override itself holds. This distinction is the whole point: comparing for
exact equality with the override's own value can never resolve a commit the backend clamped or
rejected outright (the source value would just never equal the override), and is deaf to an
unrelated undo/Reset landing mid-wait (that changes the source value to something else entirely,
which an equality check against the override doesn't recognize as "the override is stale now"
either) - both leave the override stuck showing a value nothing backs, forever.

`eq` compares two source values - defaults to `Object.is` (correct for the plain numbers `Slider`
uses); pass a custom one for a structural type, e.g. `Stage`'s `[number, number] | null` aim point
(`null` must count as equal only to `null`, and a tuple compares component-wise).

### Known limit

If a commit is rejected/fails SILENTLY - the source value never changes at all, not even to
something else - there is no external signal for this predicate (or its callers) to detect, and
the override stays stuck. This is a structural limit of comparing against a source-of-truth value
with no separate error channel, not something either caller works around.

### Used by

- `Slider` (`src/editor/controls/fields/Slider.tsx`) - clears `dragValue` (numbers, default `eq`).
- `Stage` (`src/editor/stage/Stage.tsx`) - clears `liveAim` (`[number, number] | null`, a custom
  `aimPointEq`).
