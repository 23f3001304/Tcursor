# src/editor/hooks/useEditorData.ts

All the editor's preview/timeline data fetching: the `EditDoc` itself, the camera/layout/click tracks, cursor sprites, background image, timeline media (thumbnails/waveforms/preview audio), the video/proxy source, and export-progress event subscriptions. Split out of `Editor` so the component itself only holds render + mutation logic - `Editor` composes this hook's return value with `useEditHistory`/`useTrimActions`/`useMoveModeGuard` and its own local UI state.

## useEditorData

```ts
export function useEditorData(folder: string, rev: number, quality: number): {
  doc: EditDoc | null; setDoc: (d: EditDoc) => void;
  track: CamSample[]; layout: PreviewLayout | null; layoutPresets: LayoutPresets | null;
  clicks: ClickSample[]; bgUrl: string;
  cursorSpr: CursorSpriteDto[]; cursorKnd: CursorKindSample[]; osCursor: boolean;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean; audioUrl: string; srcUrl: string;
  playing: boolean; setPlaying: (p: boolean) => void;
  exporting: boolean; setExporting: (e: boolean) => void;
  pct: number; setPct: (p: number) => void;
  exportDone: boolean; setExportDone: (d: boolean) => void;
  exportError: string | null; setExportError: (e: string | null) => void;
  exportPath: string; setExportPath: (p: string) => void;
  retryMedia: () => void;
}
```

### Inputs

- `folder: string` - project directory; the key nearly every fetch below is scoped by.
- `rev: number` - bumped by `Editor` after every `applyEditOp` (except ops whose name ends in `_effect` - see `Editor.md`). Fetches that depend on the timeline (`cameraTrack`, `previewLayout`, `previewLayouts`) are keyed on `[folder, rev]` so they refetch after an edit; fetches that don't change with generic edits (click track, cursor kinds, timeline media) are keyed on `[folder]` alone so unrelated edits don't re-trigger them.
- `quality: number` - the proxy height the Transport quality toggle cycles through (480/720/1080); keys the proxy-source effect.

### Returns

A flat object of state + setters that `Editor` destructures and threads down to `Stage`/`Transport`/`Timeline`/`EditorPanels`. Most are plain `useState` pairs (`doc`/`setDoc`, `playing`/`setPlaying`, `exporting`/`setExporting`, `pct`/`setPct`, `exportDone`/`setExportDone`, `exportError`/`setExportError`, `exportPath`/`setExportPath`); the rest (`track`, `layout`, `layoutPresets`, `clicks`, `bgUrl`, `cursorSpr`, `cursorKnd`, `osCursor`, `thumbs`, `waves`, `wavesReady`, `audioUrl`, `srcUrl`) are read-only, populated by the effects below. `retryMedia` is a function, not paired state - see Behavior.

### Behavior

**Doc load.** `getEdit(folder)` fetches the doc once per `[folder]`; the same effect calls `setCapturable(true)` so the editor window becomes screen-capturable. Failures are swallowed (`.catch(() => {})`) - `doc` simply stays `null`, and `Editor` renders its "Loading edit..." guard until it resolves.

**Preprocessed-project detection.** `getProjectManifest(folder)` resolves into ONE state object `manifest: { ready: boolean; preprocessed: boolean }` - `preprocessed` is whether `preprocess_project` already built this project's editor-preview media, `ready` is whether that read has resolved at all (`preprocessed` defaults to `false` until it has). Internal-only (not returned). Kept as a single object rather than two separate `useState` pairs specifically so the proxy-source effect below can key its dependency array on `manifest` alone: keying on `preprocessed` alone missed the case where a legacy/un-preprocessed project (or a corrupt/failed manifest read) resolves `preprocessed: false` - the same value as the pre-read default - so only `ready` actually flips, an object identity change the effect always sees even though the primitive it cares about didn't change. `manifest.ready` gates the proxy-source effect - without it, a preprocessed project would briefly load the raw 4K `video.mp4` before swapping to its proxy, a perceptible "first load feels laggy" flash.

**Timeline-dependent tracks (`[folder, rev]`).** `cameraTrack` -> `track`, `previewLayout` -> `layout`, and `previewLayouts` -> `layoutPresets` all refetch whenever `rev` bumps, since each depends on the current edit state (zooms, layout segments, aspect).

**Timeline-independent tracks (`[folder]`).** `clickTrack` -> `clicks`, `cursorKinds` -> `cursorKnd` and `osCursorInVideo` -> `osCursor` are immutable per recording, so they fetch once per folder rather than on every `rev` bump (refetching them on every edit was part of the earlier add-effect lag). `osCursor` starts at `true` - "the video already has the OS cursor", the answer that preserves today's behavior while the fetch is in flight - and tells `Stage`/`CursorPanel` whether the `System` cursor style has to be re-created from the recorded path.

**Background (`[folder, JSON.stringify(doc?.settings.background)]`).** `previewBg` -> `bgUrl` refetches only when the doc's own `background` settings change - the only kind of edit that can actually alter what `preview_bg` returns.

**Cursor sprites (`[folder, doc?.settings.cursor.pack]`).** `cursorSprites` -> `cursorSpr` refetches only when the selected pack changes (picking a different pack, or importing one, in `CursorPanel`) - not on generic `rev` bumps, so unrelated edits don't re-decode sprites.

**Timeline media (`[folder]`).** `ensureThumbs` (16 thumbnails), `ensureWaveform` (`"system"` and `"mic"`), and `ensurePreviewAudio` populate `thumbs`, `waves`, and `audioUrl` once per folder, each wrapped in `fileSrc`. `wavesReady` starts `false` on every folder change and flips `true` once the `ensureWaveform` `Promise.all` SETTLES (success or failure, via `.finally`) - `Filmstrip`/`AudioTrack` need this distinction: `waves.system`/`waves.mic` are both `""` whether the fetch simply hasn't resolved yet OR it resolved and this project genuinely has no audio for that source, so `waves` alone can't tell "still loading" from "confirmed absent" apart (unlike `thumbs`, where an empty array unambiguously means "still loading" - a real recording always eventually has at least one frame).

**Proxy source (`[folder, quality, manifest, reloadTick]`).** Pauses playback first (`setPlaying(false)`) so a `src` remount can't restart playback from 0, and resets an internal "proxy ready" ref when `folder` itself changes. Waits for `manifest.ready` before doing anything, so a preprocessed project never briefly loads raw 4K. The actual branching (raw fast-path vs. known-proxy-path vs. `ensureProxy` fetch) is `planProxySrc` (`editorData.ts`), a pure helper the effect just applies: an `immediate` filename sets `srcUrl` right away as a fast-load placeholder, a `known` filename IS the final source (no transcode - guaranteed already on disk), and `fetch: true` calls `ensureProxy(folder, quality)` and hot-swaps `srcUrl` once it resolves.

**`retryMedia` / `reloadTick`.** `retryMedia = () => setReloadTick((t) => t + 1)`; `reloadTick` is in the proxy effect's own deps, so calling `retryMedia` re-runs it even though `folder`/`quality`/`manifest` haven't changed - useful on the `fetch` branch (a genuinely fresh `ensureProxy` attempt: the case a corrupted/still-writing proxy file could plausibly resolve on a second try). On the `immediate`/`known` branches, `planProxySrc` resolves to the exact same filename every time, so re-running the effect alone does NOT force a reload (`setSrcUrl` with an unchanged string is a React no-op) - `Stage`'s Retry handler additionally calls `screen.current?.load()` directly on the `<video>` element to cover that case (see `Stage.md`).

**Every IPC-driven effect above uses the standard cleanup-token guard** (`let live = true; ...then((v) => { if (live) setX(v); }); return () => { live = false; };`) so a fetch whose effect has since been superseded (folder/rev/quality changed again, or the component unmounted, before the response landed) cannot clobber newer state - without it, rapid slider drags or a fast folder switch could apply whichever IPC response happened to land LAST over the network/IPC boundary, not whichever was requested most recently.

**Export events (`[]`).** Subscribes once to `export-progress`/`export-done`/`export-error`, updating `pct`/`exporting`/`exportDone`/`exportError`; unsubscribes on unmount. `export-done`'s payload is the exported file's own absolute path (`<folder>/final.<ext>` - see `run.rs`), stored into `exportPath` so `ExportDialog`/`ExportProgress` can offer "Show in folder" without re-deriving the output filename.

### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller; every returned field is destructured and either rendered directly or threaded into `Stage`/`Transport`/`Timeline`/`EditorPanels`/`ExportDialog`.
