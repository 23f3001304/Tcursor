/** Pure decision for the proxy-source effect in `useEditorData`: given the current quality
 *  toggle, the manifest's `preprocessed` flag, and whether a proxy has already loaded once this
 *  folder, decide (a) whether to show the raw capture immediately as a fast-load placeholder,
 *  (b) whether a known-on-disk proxy path can be used directly (no transcode), or (c) whether
 *  `ensureProxy` must be called. Filenames are relative (no `folder` prefix) - the hook joins them
 *  via `fileSrc`. Extracted so the branching (three overlapping conditions in the original inline
 *  effect) has a name and is testable without mocking IPC/React state. */
export interface ProxyPlan {
  /** Non-null: set `srcUrl` to this filename right away (raw fast-load placeholder). */
  immediate: string | null;
  /** Non-null: this IS the final source - a known-preprocessed proxy already on disk, no fetch. */
  known: string | null;
  /** Whether to call `ensureProxy(folder, quality)` and hot-swap once it resolves. */
  fetch: boolean;
}

export function planProxySrc(
  quality: number,
  defaultHeight: number,
  preprocessed: boolean,
  proxyReady: boolean,
): ProxyPlan {
  // Raw fast-path ONLY for a not-yet-preprocessed project with no proxy loaded yet - show
  // something while the proxy transcodes. A preprocessed project skips straight to its proxy
  // (never shows raw 4K); once a proxy HAS loaded for this folder, a later quality switch keeps
  // showing it instead of flashing back to raw video (see `proxyReadyRef` in the hook).
  const immediate = !proxyReady && !preprocessed ? "video.mp4" : null;
  // Skip the lazy transcode entirely at the DEFAULT quality on an already-preprocessed project -
  // that exact proxy is guaranteed to already be on disk. Any OTHER quality (the in-editor
  // quality toggle), or a legacy/un-preprocessed project, falls through to `ensureProxy`.
  if (preprocessed && quality === defaultHeight) {
    return { immediate, known: `preview_${quality}_rt.mp4`, fetch: false };
  }
  return { immediate, known: null, fetch: true };
}
