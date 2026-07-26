# src/editor/Editor.tsx

The post-record editor: loads `edit.json` via IPC, owns all shared editor state, and composes `TopBar`, `Rail`, an inspector/panel, `Stage`, `Transport`, and `Timeline`. The preview plays the recording natively (`Stage`) and applies the exact camera curve (`cameraTrack`) as a transform, so it is smooth 60fps; edits go through the Edit API and `rev` bumps so the camera curve refetches.

## Editor

```tsx
export function Editor({ folder, onClose }: { folder: string; onClose: () => void }): JSX.Element
```

Renders the editor layout and manages the editing lifecycle for one recording.

### Props

- `folder: string` - absolute path to the project directory. *Why:* every IPC call (load, preview tracks, autoedit, export) takes the folder as its key.
- `onClose: () => void` - return to the HUD. *Why:* the editor does not own the window/view switch; the parent does.

### Behavior

**Preview data (effects keyed per source).**
- `cameraTrack` and `previewLayout` are fetched on `[folder, rev]` - they depend on the timeline, so they refetch when an edit bumps `rev`.
- `clickTrack`, `spotlightHolds`, and `previewBg` are fetched on `[folder]` only - clicks and recorded holds are immutable and the background doesn't change with the current op set, so refetching them on every edit was part of the add-effect lag and was removed.
- `cursorKinds` and the timeline media (`ensureThumbs`, `ensureWaveform` x2, `ensurePreviewAudio`) are fetched once per `[folder]`. `cursorSprites` is fetched on `[folder, doc?.settings.cursor.pack]` instead of `rev` - it re-decodes only when the selected cursor pack actually changes (picking a different pack, or importing one, in `CursorPanel`), not on every unrelated edit.
- `getProjectManifest` is fetched once per `[folder]` into `preprocessed` - whether `preprocess_project` (run from the HUD's "Saving..." step right after a recording stops) already built this project's editor-preview media. It gates the proxy fetch below. The other timeline-media calls (`ensureThumbs`/`ensureWaveform`/`ensurePreviewAudio`) stay unconditional regardless of `preprocessed` - they are `generate_once`-cached on the Rust side and already cheap once preprocessing has produced their files; only the proxy transcode is expensive AND unambiguous enough (no "was this track even recorded" guessing) to skip outright on the frontend.

**Fast load (`[folder, quality]`).** On an already-`preprocessed` project AT the default quality (`DEFAULT_PROXY_HEIGHT`, 720), points `srcUrl` straight at the known proxy path - no transcode, no `ensureProxy` round trip. Otherwise (a legacy/un-preprocessed project, a failed preprocessing pass, or the user picked a non-default quality via the Transport toggle) shows the raw capture instantly (`video.mp4`) so the preview appears with no transcode wait, then hot-swaps `srcUrl` to the light proxy once `ensureProxy` finishes (for smoother scrubbing). Playback is paused first so the `src` remount can't restart from 0.

**Playback.** Driven by the native `<video>` in `Stage`, which reports its time via `onTime`; `onTime` sets `timeMs` and stops playback at `resolveTrim(doc?.trim, dur).outMs` - the trim's resolved out point, not just the clip end, so play never runs into a trimmed-out tail. There is no RAF playhead loop in the editor. `dur` is the `<video>`'s real duration (`vidDurMs`) when known, else `doc.trim.out_ms` - the encoded length can differ from the seeded timeline span, so the video's own duration is the source of truth for the playhead end. Scrubbing the timeline itself is unrestricted (only `onPlay`, below, respects the trim start).

**Trim + aspect (passed to `Transport`).** `trimRange = resolveTrim(doc.trim, dur)` and `trimmed = trimRange.inMs > 0 || trimRange.outMs < dur` are computed once per render and passed down as `trimmed`/`onResetTrim` (clears `SetTrim` back to `{0,0}`). `Transport`'s `onPlay` wrapper snaps `timeMs` to `trimRange.inMs` first when starting playback from outside `[inMs, outMs)`. `doc.aspect`/`SetAspect` are passed straight through as `aspect`/`onAspect`. The actual trim RANGE is set by dragging `TrimOverlay`'s edge handles on the `Timeline`, not from `Transport`.

**Export events (`[]`, via `useEditorData`).** Subscribes to `export-progress` / `export-done` / `export-error`; unsubscribes on unmount. `export-done` sets `exportDone`; `export-error` captures the payload into `exportError` (both cleared back to their "no outcome yet" state whenever a new export starts). `TopBar`'s Export button (`onOpenExport`) only opens `showExportDialog` - the actual `exportProject(folder, settings)` call, and the `exporting`/`pct`/`exportDone`/`exportError` reset, happen in the `onExport` callback passed to `ExportDialog`.

**`applyOp(op)`.** Persists one edit op via `applyEditOp`, swaps in the returned doc, and bumps `rev` - except for ops whose name ends in `_effect` (add/update/remove effect), which skip the bump because effects don't change the camera/layout/clicks/bg (this is why adding or dragging a spotlight is instant rather than laggy; the preview reflects effects straight from the returned doc).

**`addZoom` / `addSpotlight` / `addCameraMove`.** Add a 2s zoom / spotlight, or a centered camera-move keyframe (`x:0.5, y:0.5, size:0.25`), at the playhead and select it.

**`zoomAt(x, y)`.** The click-to-zoom handler passed to `Stage`: pauses, adds a full zoom (`add_zoom_full`), then sets its target to the clicked 0..1 screen point (`update_zoom` with `{ fixed: { x, y } }`) and selects it.

**`onRun()`.** Runs the AI director (`aiAutoedit`), replaces `doc`, and bumps `rev`; errors are swallowed so a missing Ollama server doesn't crash the editor.

**Loading guard.** Renders "Loading edit..." until `doc` resolves.

**Layout.** `ResizeEdges` (frameless-window grips), `TopBar`, then `e-body` containing `Rail`, the active inspector/panel (`ZoomInspector` when a zoom is selected, `EffectInspector` when an effect is selected, `LayoutInspector` when a layout segment is selected, `CameraMoveInspector` when a camera-move keyframe is selected, `AiPanel` on the AI tab, else a stub panel), and `Stage`; then `Transport`, `Timeline`, `moveOffDialog`, and `ExportDialog`.

### Notes

- `selZoom` / `selEffect` / `selLayout` / `selCamMove` are derived from `sel` against `doc.zooms` / `doc.effects` / `doc.layout` / `doc.camera_moves` to choose which inspector to show.
- `quality` cycles 480→720→1080 and re-runs the proxy effect (initialized to `DEFAULT_PROXY_HEIGHT`, the same 720 `preprocess_project` transcodes ahead of time); `muted` toggles the preview `<audio>`.
- `proj` is the last path segment of `folder`, shown by `TopBar`.
- `showExportDialog` is the only export-dialog state `Editor` owns itself; `exporting`/`pct`/`exportDone`/`exportError` are all lifted from `useEditorData` and passed straight through to `ExportDialog`.
