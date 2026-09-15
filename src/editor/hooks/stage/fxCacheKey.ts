export function timeBucket(t: number, bucketMs: number): number {
  return Math.round(t / bucketMs) * bucketMs;
}

export function spotParamsKey(
  resolved: {
    dim: number;
    radius: number;
    feather: number;
    mode: string;
    tint: [number, number, number];
  } | null,
  screenScale: number,
  separable: boolean,
): string {
  if (!resolved) return "off";
  return (
    `${resolved.dim}-${resolved.radius}-${resolved.feather}-${resolved.mode}-${resolved.tint.join(",")}` +
    `-${screenScale.toFixed(3)}-${separable ? "sep" : "live"}`
  );
}

export function fxCacheKey(
  tBucket: number,
  cursorStr: string,
  spotParamsStr: string,
  clicksStr: string,
  fxParamsStr: string,
  camStr: string,
): string {
  return `${tBucket}_${cursorStr}_${spotParamsStr}_${clicksStr}_${fxParamsStr}_${camStr}`;
}

export function isStaleFxResponse(requestedKey: string, wantedKey: string): boolean {
  return requestedKey !== wantedKey;
}

export type FxResponseAction = { kind: "stale" } | { kind: "apply"; imageUrl: string | null };

export function fxResponseAction(
  requestedKey: string,
  wantedKey: string,
  imageUrl: string | null,
): FxResponseAction {
  if (isStaleFxResponse(requestedKey, wantedKey)) return { kind: "stale" };
  return { kind: "apply", imageUrl };
}
