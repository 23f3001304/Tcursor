/** Whether a gesture-scoped optimistic override (`Slider`'s `dragValue`, `Stage`'s reticle-drag
 *  `liveAim`) should clear.
 *
 *  Clears on the FIRST change away from `settled` (the source-of-truth value as of the moment the
 *  gesture began) - whatever it changes TO - rather than requiring the source value to land back
 *  on the exact value the override itself holds. Comparing for exact equality with the override
 *  can never resolve a commit the backend clamped or rejected outright (the source value would
 *  just never equal the override), and is deaf to an unrelated undo/Reset landing mid-wait (that
 *  changes the source value to something else entirely, which an equality check against the
 *  override doesn't recognize as "the override is stale now" either) - both leave the override
 *  stuck showing a value nothing backs, forever.
 *
 *  `eq` compares two source values - defaults to `Object.is` (numbers); pass a custom one for a
 *  structural type (e.g. `Stage`'s `[number, number] | null` aim point). Plain function, no
 *  React - unit-testable directly. */
export function shouldClearOverride<T>(
  gestureActive: boolean,
  overrideSet: boolean,
  current: T,
  settled: T,
  eq: (a: T, b: T) => boolean = Object.is,
): boolean {
  return !gestureActive && overrideSet && !eq(current, settled);
}
