# src/editor/hooks/editorData.ts

Pure helpers extracted from `useEditorData` so its trickier branching has a name and is unit-testable without mocking IPC/React state.

## planProxySrc

```ts
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
): ProxyPlan
```

Decides what the proxy-source effect should do this run: show the raw capture immediately, point straight at a known-on-disk proxy, and/or call `ensureProxy`.

### Inputs

- `quality: number` - the current proxy height (the Transport quality toggle: 480/720/1080).
- `defaultHeight: number` - `DEFAULT_PROXY_HEIGHT`; the quality `preprocess_project` transcodes ahead of time.
- `preprocessed: boolean` - whether `preprocess_project` already built this project's proxy media (from the project manifest).
- `proxyReady: boolean` - whether a proxy has already loaded once for the CURRENT folder (`proxyReadyRef.current` in the hook) - once true, a later quality switch keeps showing the current proxy instead of flashing back to raw video.

### Returns

`ProxyPlan` - filenames are relative (no `folder` prefix); the caller joins them via `fileSrc`.

- `immediate: string | null` - non-null (`"video.mp4"`) only for a not-yet-preprocessed project with no proxy loaded yet: set `srcUrl` to this right away as a fast-load placeholder while the real proxy transcodes.
- `known: string | null` - non-null (`"preview_${quality}_rt.mp4"`) only when `preprocessed && quality === defaultHeight`: this IS the final source, guaranteed already on disk - no transcode needed.
- `fetch: boolean` - `true` whenever `known` is null: the caller must call `ensureProxy(folder, quality)` and hot-swap `srcUrl` once it resolves (a legacy/un-preprocessed project, a failed preprocessing pass, or a non-default quality).

### Used by

`useEditorData` (`src/editor/hooks/useEditorData.ts`) - the proxy-source effect, gated on `manifest.ready` before calling this.
