export function shouldClearOverride<T>(
  gestureActive: boolean,
  overrideSet: boolean,
  current: T,
  settled: T,
  eq: (a: T, b: T) => boolean = Object.is,
): boolean {
  return !gestureActive && overrideSet && !eq(current, settled);
}
