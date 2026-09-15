# src/editor/hooks/doc/useEditorData.ts

All the editor's preview/timeline data fetching: the `EditDoc` itself, the camera/layout/click tracks, and - through three sibling hooks it composes - the cursor layer (`useCursorData`), the background (`usePreviewBg`) and the timeline media plus the proxy source (`useMediaAssets`). Split out of `Editor` so the component itself only holds render + mutation logic. The three siblings are subsystems with their own fetch keys and their own failure modes, not slices of one list; this file keeps the doc and the tracks that key on `rev`, and spreads the siblings' returns into one flat object so no caller has to know they exist. Export-run state lives in `useExportState.ts`; `Editor.tsx` calls both hooks separately.

## useEditorData

```ts
export function useEditorData(folder: string, rev: number, quality: number): {
  doc: EditDoc | null; setDoc: (d: EditDoc) => void;
  track: CamSample[]; layout: PreviewLayout | null; layoutPresets: LayoutPresets | null;
  clicks: ClickSample[]; bg: StageBg;
  cursorSpr: CursorPackDto | null; cursorKnd: CursorKindSample[]; cursorLyr: CursorLayerDto | null; osCursor: boolean;
  thumbs: string[]; waves: { system: string; mic: string }; wavesReady: boolean; audioUrl: string; srcUrl: string;
  playing: boolean; setPlaying: (p: boolean) => void;
  retryMedia: () => void;
}
```

### Inputs

- `folder: string` - project directory; the key nearly every fetch below is scoped by.
- `rev: number` - bumped by `Editor` after every `applyEditOp` (except ops whose name ends in `_effect` - see `Editor.md`). Fetches that depend on the timeline (`cameraTrack`, `previewLayout`, `previewLayouts`) are keyed on `[folder, rev]` so they refetch after an edit; fetches that don't change with generic edits (click track, cursor kinds, timeline media) are keyed on `[folder]` alone so unrelated edits don't re-trigger them.
- `quality: number` - the proxy height the Transport quality toggle cycles through (480/720/1080); keys the proxy-source effect.

### Returns

A flat object of state + setters that `Editor` destructures and threads down to `Stage`/`Transport`/`Timeline`/`EditorPanels`. Most are plain `useState` pairs (`doc`/`setDoc`, `playing`/`setPlaying`); the rest (`track`, `layout`, `layoutPresets`, `clicks`, `bg`, `cursorSpr`, `cursorKnd`, `cursorLyr`, `osCursor`, `thumbs`, `waves`, `wavesReady`, `audioUrl`, `srcUrl`) are read-only, populated by the effects below. `retryMedia` is a function, not paired state - see Behavior.

### Behavior

**Doc load.** `getEdit(folder)` fetches the doc once per `[folder]`; the same effect calls `setCapturable(true)` so the editor window becomes screen-capturable. Failures are swallowed (`.catch(() => {})`) - `doc` simply stays `null`, and `Editor` renders its "Loading edit..." guard until it resolves.

**Timeline-dependent tracks (`[folder, rev]`).** `cameraTrack` -> `track`, `previewLayout` -> `layout`, and `previewLayouts` -> `layoutPresets` all refetch whenever `rev` bumps, since each depends on the current edit state (zooms, layout segments, aspect).

**Timeline-independent tracks (`[folder]`).** `clickTrack` -> `clicks` is immutable per recording, so it fetches once per folder rather than on every `rev` bump (refetching it on every edit was part of the earlier add-effect lag). The cursor track and layer follow the same rule in `useCursorData`.

**The three composed hooks.** `usePreviewBg(folder, doc)` returns the `bg: StageBg`, `useCursorData(folder, doc)` the four `cursor*`/`osCursor` fields, and `useMediaAssets(folder, quality, setPlaying)` the thumbs, waveforms, preview audio, proxy `srcUrl` and `retryMedia`. Each has its own page. `playing`/`setPlaying` stay here because they are not a fetched asset - `useMediaAssets` only borrows the setter, to pause playback before a source swap.

**Every IPC-driven effect above uses the standard cleanup-token guard** (`let live = true; ...then((v) => { if (live) setX(v); }); return () => { live = false; };`) so a fetch whose effect has since been superseded (folder/rev/quality changed again, or the component unmounted, before the response landed) cannot clobber newer state - without it, rapid slider drags or a fast folder switch could apply whichever IPC response happened to land LAST over the network/IPC boundary, not whichever was requested most recently.

### Used by

`Editor` (`src/editor/Editor.tsx`) - one of two callers alongside `useExportState` (Task 11); every returned field is destructured and either rendered directly or threaded into `Stage`/`Transport`/`Timeline`/`EditorPanels`.
