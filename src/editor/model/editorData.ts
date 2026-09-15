import type { PreviewLayout } from "../../shared/ipc";

export interface ProxyPlan {
  immediate: string | null;
  known: string | null;
  fetch: boolean;
}

export function planProxySrc(
  quality: number,
  defaultHeight: number,
  preprocessed: boolean,
  proxyReady: boolean,
): ProxyPlan {
  const immediate = !proxyReady && !preprocessed ? "video.mp4" : null;
  if (preprocessed && quality === defaultHeight) {
    return { immediate, known: `preview_${quality}_rt.mp4`, fetch: false };
  }
  return { immediate, known: null, fetch: true };
}

export function hasWebcamSignal(layout: PreviewLayout | null): boolean {
  return layout != null && layout.cam != null;
}
