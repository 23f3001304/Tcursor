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
- `cursorSprites`/`cursorKinds` and the timeline media (`ensureThumbs`, `ensureWaveform` x2, `ensurePreviewAudio`) are fetched once per `[folder]`.

**Fast load (`[folder, quality]`).** Shows the raw capture instantly (`video.mp4`) so the preview appears with no transcode wait, then hot-swaps `srcUrl` to the light proxy once `ensureProxy` finishes (for smoother scrubbing). Playback is paused first so the `src` remount can't restart from 0.

**Playback.** Driven by the native `<video>` in `Stage`, which reports its time via `onTime`; `onTime` sets `timeMs` and stops playback at `dur`. There is no RAF playhead loop in the editor. `dur` is the `<video>`'s real duration (`vidDurMs`) when known, else `doc.trim.out_ms` - the encoded length can differ from the seeded timeline span, so the video's own duration is the source of truth for the playhead end.

**Export events (`[]`).** Subscribes to `export-progress` / `export-done` / `export-error`; unsubscribes on unmount.

**`applyOp(op)`.** Persists one edit op via `applyEditOp`, swaps in the returned doc, and bumps `rev` - except for ops whose name ends in `_effect` (add/update/remove effect), which skip the bump because effects don't change the camera/layout/clicks/bg (this is why adding or dragging a spotlight is instant rather than laggy; the preview reflects effects straight from the returned doc).

**`addZoom` / `addSpotlight`.** Add a 2s zoom / spotlight at the playhead and select it.

**`zoomAt(x, y)`.** The click-to-zoom handler passed to `Stage`: pauses, adds a full zoom (`add_zoom_full`), then sets its target to the clicked 0..1 screen point (`update_zoom` with `{ fixed: { x, y } }`) and selects it.

**`onRun()`.** Runs the AI director (`aiAutoedit`), replaces `doc`, and bumps `rev`; errors are swallowed so a missing Ollama server doesn't crash the editor.

**Loading guard.** Renders "Loading edit..." until `doc` resolves.

**Layout.** `ResizeEdges` (frameless-window grips), `TopBar`, then `e-body` containing `Rail`, the active inspector/panel (`ZoomInspector` when a zoom is selected, `EffectInspector` when an effect is selected, `AiPanel` on the AI tab, else a stub panel), and `Stage`; then `Transport` and `Timeline`.

### Notes

- `selZoom` / `selEffect` are derived from `sel` against `doc.zooms` / `doc.effects` to choose which inspector to show.
- `quality` cycles 480→720→1080 and re-runs the proxy effect; `muted` toggles the preview `<audio>`.
- `proj` is the last path segment of `folder`, shown by `TopBar`.
