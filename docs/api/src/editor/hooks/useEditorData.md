# src/editor/hooks/useEditorData.ts

All the editor's preview/timeline data fetching: the `EditDoc` itself, the camera/layout/click tracks, cursor sprites, background image, timeline media (thumbnails/waveforms/preview audio), the video/proxy source, and export-progress event subscriptions. Split out of `Editor` so the component itself only holds render + mutation logic - `Editor` composes this hook's return value with `useEditHistory`/`useTrimActions`/`useMoveModeGuard` and its own local UI state.

## useEditorData

```ts
export function useEditorData(folder: string, rev: number, quality: number): {
  doc: EditDoc | null; setDoc: (d: EditDoc) => void;
  track: CamSample[]; layout: PreviewLayout | null; layoutPresets: LayoutPresets | null;
  clicks: ClickSample[]; bgUrl: string;
  cursorSpr: CursorSpriteDto[]; cursorKnd: CursorKindSample[];
  thumbs: string[]; waves: { system: string; mic: string }; audioUrl: string; srcUrl: string;
  playing: boolean; setPlaying: (p: boolean) => void;
  exporting: boolean; setExporting: (e: boolean) => void;
  pct: number; setPct: (p: number) => void;
  exportDone: boolean; setExportDone: (d: boolean) => void;
  exportError: string | null; setExportError: (e: string | null) => void;
}
```

### Inputs

- `folder: string` - project directory; the key nearly every fetch below is scoped by.
- `rev: number` - bumped by `Editor` after every `applyEditOp` (except ops whose name ends in `_effect` - see `Editor.md`). Fetches that depend on the timeline (`cameraTrack`, `previewLayout`, `previewLayouts`) are keyed on `[folder, rev]` so they refetch after an edit; fetches that don't change with generic edits (click track, cursor kinds, timeline media) are keyed on `[folder]` alone so unrelated edits don't re-trigger them.
- `quality: number` - the proxy height the Transport quality toggle cycles through (480/720/1080); keys the proxy-source effect.

### Returns

A flat object of state + setters that `Editor` destructures and threads down to `Stage`/`Transport`/`Timeline`/`EditorPanels`. Most are plain `useState` pairs (`doc`/`setDoc`, `playing`/`setPlaying`, `exporting`/`setExporting`, `pct`/`setPct`, `exportDone`/`setExportDone`, `exportError`/`setExportError`); the rest (`track`, `layout`, `layoutPresets`, `clicks`, `bgUrl`, `cursorSpr`, `cursorKnd`, `thumbs`, `waves`, `audioUrl`, `srcUrl`) are read-only, populated by the effects below.

### Behavior

**Doc load.** `getEdit(folder)` fetches the doc once per `[folder]`; the same effect calls `setCapturable(true)` so the editor window becomes screen-capturable. Failures are swallowed (`.catch(() => {})`) - `doc` simply stays `null`, and `Editor` renders its "Loading edit..." guard until it resolves.

**Preprocessed-project detection.** `getProjectManifest(folder)` resolves into `preprocessed` (whether `preprocess_project` already built this project's editor-preview media) and `manifestReady` (whether that read has resolved at all; `preprocessed` defaults to `false` until it has). Both are internal-only (not returned). `manifestReady` exists specifically to gate the proxy-source effect below - without it, a preprocessed project would briefly load the raw 4K `video.mp4` before swapping to its proxy, a perceptible "first load feels laggy" flash.

**Timeline-dependent tracks (`[folder, rev]`).** `cameraTrack` -> `track`, `previewLayout` -> `layout`, and `previewLayouts` -> `layoutPresets` all refetch whenever `rev` bumps, since each depends on the current edit state (zooms, layout segments, aspect).

**Timeline-independent tracks (`[folder]`).** `clickTrack` -> `clicks` and `cursorKinds` -> `cursorKnd` are immutable per recording, so they fetch once per folder rather than on every `rev` bump (refetching them on every edit was part of the earlier add-effect lag).

**Background (`[folder, JSON.stringify(doc?.settings.background)]`).** `previewBg` -> `bgUrl` refetches only when the doc's own `background` settings change - the only kind of edit that can actually alter what `preview_bg` returns.

**Cursor sprites (`[folder, doc?.settings.cursor.pack]`).** `cursorSprites` -> `cursorSpr` refetches only when the selected pack changes (picking a different pack, or importing one, in `CursorPanel`) - not on generic `rev` bumps, so unrelated edits don't re-decode sprites.

**Timeline media (`[folder]`).** `ensureThumbs` (16 thumbnails), `ensureWaveform` (`"system"` and `"mic"`), and `ensurePreviewAudio` populate `thumbs`, `waves`, and `audioUrl` once per folder, each wrapped in `fileSrc`.

**Proxy source (`[folder, quality, preprocessed]`).** Pauses playback first (`setPlaying(false)`) so a `src` remount can't restart playback from 0, and resets an internal "proxy ready" ref when `folder` itself changes. Waits for `manifestReady` before doing anything, so a preprocessed project never briefly loads raw 4K. Then: if there's no ready proxy yet AND the project isn't preprocessed, shows the raw capture (`video.mp4`) immediately as a fast-load placeholder; if the project IS preprocessed and `quality` is exactly `DEFAULT_PROXY_HEIGHT`, points straight at the known proxy path (`preview_${quality}_rt.mp4`) with no transcode - that exact file is guaranteed to already be on disk; otherwise falls through to `ensureProxy(folder, quality)` (a legacy/un-preprocessed project, a failed preprocessing pass, or a non-default quality the user picked via the Transport toggle), hot-swapping `srcUrl` once the transcode finishes.

**Export events (`[]`).** Subscribes once to `export-progress`/`export-done`/`export-error`, updating `pct`/`exporting`/`exportDone`/`exportError`; unsubscribes on unmount.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller; every returned field is destructured and either rendered directly or threaded into `Stage`/`Transport`/`Timeline`/`EditorPanels`/`ExportDialog`.
