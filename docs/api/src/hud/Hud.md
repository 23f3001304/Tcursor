# src/hud/Hud.tsx

Root component of the floating recorder bar. Owns device selection, export flags, and the active panel directly; delegates the record/stop/preprocess lifecycle to `useRecordingFlow`. Wires together every visual region: the titlebar, device dropdowns, waveform meter, timer, and the animated overlay panels for Settings and Preferences. This is the single stateful hub; all child components are controlled.

## Hud

Renders the full HUD window content and manages the recording lifecycle.

### Props

- `onEdit?: (folder: string) => void` - optional callback invoked after `stopRecording` succeeds, with the project folder path. *Why:* the stop path no longer automatically kicks off the export pipeline; instead it hands the folder to the caller (`App`) so it can open the editor. When `onEdit` is not provided the recorder operates in standalone mode and the export is not triggered.

### Behavior

**Window constant.** `WIDTH = 980` is the fixed horizontal size of the bar window. The box dimensions (BOX_W=360, BOX_H=440) are local constants used by both the open and restore transitions.

**State and hooks.**
- `displays / mics / sel / setSel` - enumerated devices and current selection from `useDevices`, which polls `listDisplays` and `listAudioInputs` via IPC on mount. *Why a hook:* keeps enumeration and refresh logic out of this file.
- `camId` - selected camera device id, or `null` to mean "first available". *Why nullable:* the browser does not guarantee a stable device id across sessions.
- `camOn` - whether the camera preview and recording are active. *Why separate from camId:* lets the user keep a camera selected but temporarily disabled without losing the selection.
- `cam` - live preview handle from `useWebcamPreview(camId, camOn)`. Exposes `cam.ref` (bound to the `<video>` element), `cam.on` (whether a stream is active), and `cam.stream()` (the `MediaStream` for the recorder).
- `cameras` - list of detected camera devices from `useCameraDevices`. Re-enumerates when `cam.on` flips (the numeric argument forces a re-query tick). *Why on `cam.on`:* browsers only expose camera labels after permission is granted, which happens the moment the preview starts.
- `recording / paused / saving / savePct / err / toggle / togglePause` - from `useRecordingFlow` (see `docs/api/src/hud/hooks/useRecordingFlow.md`), which owns the whole record -> stop -> preprocess -> edit lifecycle. `exporting`/`pct` (below) stay local to `Hud` - they are unrelated leftover state from before export moved into the editor's `ExportDialog` (see `src/editor/Editor.tsx`); nothing in the current app flow sets `exporting` true from the HUD.
- `exporting` / `pct` - export progress percentage, driven by `export-progress` Tauri events.
- `panel` - which overlay panel is mounted: `"settings"`, `"preferences"`, or `null`. Drives the `AnimatePresence` overlay.
- `barShown` - toggles the `.as-box` CSS class and gates the bar content subtree. *Why needed:* the window morphs to a smaller box for panels; while morphing the bar must be invisible so it does not flash underneath the incoming panel.
- `lastFolder` ref - stores the project folder path returned by `stopRecording` so the export-error listener can call `revealItemInDir` even after recording state has reset.
- `themeRef` ref - persists `{ theme, accent }` so the `matchMedia` OS-change handler can call `applyTheme` with fresh values without closing over stale state.
- `elapsed` - elapsed recording time from `useRecordingTimer(recording, paused)`.
- `levels` - mic waveform bar heights (0-1 floats) from `useMicWaveform(recording && !paused)`.

**Window sizing effect (`useEffect` on `[menu]`).**
While `barShown` is true, calls `win.setSize(barSize())` whenever a dropdown opens or closes. `barSize()` returns `LogicalSize(WIDTH, menu ? 430 : 132)` - taller when a dropdown is open so the menu list overflows into extra window space rather than being clipped by the frame. *Why gated on `barShown`:* the panel transition uses `morphWindow` for a spring animation; resizing during that animation would fight it.

**Theme effect (`useEffect` on `[]`, runs once).**
Calls `getSettings()` and immediately applies the persisted theme and accent color via `applyTheme`. Attaches a `matchMedia("prefers-color-scheme: dark")` listener so System mode stays reactive to OS changes during the session. Reads from `themeRef` (not state) to avoid a stale-closure bug. Cleans up the listener on unmount.

**Export event listeners (`useEffect` on `[]`).**
Subscribes to three Tauri events:
- `export-progress` (payload `number`) - updates `pct`.
- `export-done` (payload folder path string) - sets `exporting` false and calls `revealItemInDir` on `final.mp4`.
- `export-error` - sets `exporting` false and reveals `video.mp4` from `lastFolder` as a fallback.

All three `listen` promises return unsubscribe functions called on cleanup.

**`toggle()` / `togglePause()`.**
Both come straight from `useRecordingFlow(...)`, fed `micOn`/`sel.micId`/`sel.displayId`/`sysOn`/`gameMode`/`camOn`/`cam.stream`/`webcam`/`onEdit`/`lastFolder`. See `docs/api/src/hud/hooks/useRecordingFlow.md` for the full record -> stop -> preprocess -> edit sequence (`toggle`'s Stop half now awaits the post-record `preprocess_project` pass, reported here as `saving`/`savePct`, before calling `onEdit`). `lastFolder` itself stays a `Hud`-owned ref (passed into the hook by reference) purely so the unrelated export-error listener below can still read it.

**`openExistingProject()`.**
Calls `openProject()` (native `*.tcursor` file picker) and, on success, calls `onEdit?.(folder)` with the returned containing folder - the same prop `toggle()`'s stop path uses, so opening an existing project routes to the editor identically to finishing a fresh recording. A rejected promise (the user cancelled the dialog) is swallowed; there is nothing to surface.

**Panel transition helpers.**
- `openPanel(p)` - sets `barShown = false`, sets `panel = p`, and calls `morphWindow` to animate the window from bar dimensions to box dimensions over 200ms.
- `restoreBar()` - called from `AnimatePresence`'s `onExitComplete` after the panel exit animation finishes. Calls `morphWindow` back from box to bar dimensions, then sets `barShown = true` so the bar fades in only after the resize completes.

**Rendered regions.**
- `AnimatePresence` overlay: mounts either `<Settings>` or `<Preferences>` with a scale+opacity spring (0.97->1 in, 0.98->0 out over 180ms). `key={panel}` ensures the exit animation fires when switching panels.
- Titlebar: drag region with brand name, optional error label, and window controls. Open Project, Settings, and Preferences buttons are hidden while recording or exporting; Open Project is additionally disabled while `saving` (avoids a race between a just-finished recording's own `onEdit` call and one triggered by opening a different project mid-save).
- Grip: left drag handle (`data-tauri-drag-region`).
- Camera preview: `<video>` bound to `cam.ref`, with a camera-off icon overlay when `camOn && cam.on` is false.
- Four conditional rows for bar content, in priority order: exporting progress (`exporting`), saving/preprocessing progress (`saving` - a `Saving… {savePct}%` pill, same `.exporting` style class plus a `.saving` hook), device selectors (idle), waveform meter (recording).
- Timer, Pause/Resume button, and Record/Stop button always on the right. The Record/Stop button label reads `Saving…` (no percent - the pill above already shows it) while `saving` is true.

### Notes

- `MotionConfig reducedMotion="user"` wraps the entire tree; all Motion animations disable automatically when the OS has reduced-motion enabled.
- The Settings and Preferences buttons are conditionally rendered (not just disabled) during recording and exporting to avoid occupying titlebar space when those actions are not available.
