# src/editor/editorPanelsProps.ts

The contract between `Editor` and the panel column: every value one of the nine panels needs, in one named interface. Split out of `EditorPanels.tsx` so the router file is the tab switch and this one is the shape it switches on - the list is long because the panels are nine different things, not because any one of them takes much.

## EditorPanelsProps

```ts
export interface EditorPanelsProps { ... }
```

The full field list is in the source; every field is documented below in the order the router threads it.

### Props

- `doc: EditDoc` - the current edit document; every panel's settings read from it.
- `tab: Tab` / `setTab` - the panel to render, and the setter every close X uses to collapse the column (`setTab(null)`). `tab` is non-null here; `Editor` and `ClassicShell` hold the nullable one.
- `timeMs: number` - playhead position, passed to `CameraPanel` and `CaptionsPanel`. **Gated by the caller** (render hygiene pass): `ClassicShell` only passes the real, ticking value while `tab === "camera"` or `tab === "captions"` - every other panel doesn't read `timeMs` at all, so it passes a constant `0` instead, letting this component's `React.memo` actually skip a re-render on a tick while some other panel is showing. The captions tab needs the live value for one reason: its transcript lights the row the playhead is inside, and a row that only lit when something else re-rendered would be worse than no mark.
- `reloadDoc: () => void` (M5 T6) - re-read `edit.json` because the BACKEND wrote it, as one undo step. Only `CaptionsPanel` takes it, and only because transcription is the only backend-side writer of the doc; see `src/editor/shell/slotProps.md`.
- `sel` / `onSel` / `onSeek` (M5 T6) - the selection and the two things a transcript row click does. Only `CaptionsPanel` takes them. Note that `sel`/`setSel` were removed from this component in M1a when the inspectors moved to `PropertiesSlot`; what came back is narrower and for a different reason - this panel LISTS timeline regions, it does not edit them.
- `timeMsRef: RefObject<number>` - the SAME playhead, but as a ref that's always current regardless of `tab`. Two callers: the Effects panel's "Add layout" button (drops a segment at the playhead), which needs the TRUE current time at click time no matter which panel happens to be showing; and `LayoutsPanel`, which reads it ONCE on mount to open on the layout the playhead is inside. Neither need forces `timeMs` itself to always be live, which would defeat the gating above.
- `running` / `aiError` / `aiProgress` / `onRun` - AI Director state and the run trigger, passed straight through to `AiPanel`. `aiProgress` (`{step, total} | null`, from `Editor`'s `useAiRun()`) is the live "k of N" for the pointer replay's progress line - `null` while thinking and applying, non-null only while the replay walks the applied edits.
- `aiRun` / `aiSkipped` / `aiApplying` / `aiPreviewId` / `onToggleItem` / `onPreviewItem` / `onApplyRun` / `onDiscardRun` (M4 T4) - the AI review sheet's whole state and its four actions, passed straight through to `AiPanel` (which renames the last two to `onApply` / `onDiscard` locally). None of it is shell state: the sheet renders inside the AI panel's own body, and `stageOutline` - the one thing it draws outside - goes to `Stage` instead, never through here. Every handler is `useCallback`'d upstream, because an identity that changed per render would defeat this component's memo on every playhead tick.
- `exporting: boolean` (bug-sweep-2 Task 8, L3) - passed straight through to `AiPanel`, which locks its run button on it too, matching `Transport`'s wand and its own play/trim/aspect `locked` gate: a director pass mutating `edit.json` while the exporter renders from its own doc snapshot would silently diverge the two.
- `onAutoModel: (v: string) => void` - the QUIET model-write (no undo step; see `Editor`'s `useDocSettings`), passed straight through to `AiPanel` for its mount-time auto-default-model effect. Distinct from the Engine picker's own `onChange`, which is built inline here from `saveDocSettings` (records an undo step, since that's a deliberate user pick).
- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - the shared mutation entry point; every panel button applies an `EditOp` through this.
- `saveDocSettings: (s: EditDoc["settings"]) => void` - bulk-saves a patched `settings` object; used by panels that edit `Settings` fields directly (background, cursor, camera, hotkeys, audio) rather than emitting an `EditOp`.
- `moveMode` / `requestMoveMode` / `camDraftRef` - webcam "Move mode" state, threaded to `CameraPanel` so it can toggle drag-to-reposition and read the live unsaved pose (`CamPose` from `src/editor/stage/camera/cameraMoves.ts`). `moveMode` also reaches `ZoomInspector`, which now lives in `PropertiesSlot` and gets it from the same `SlotProps` bundle.
- `osCursorInVideo` - passed straight through to `CursorPanel`, which annotates the "System" style when the recording has no baked OS cursor to show (see its own doc).
- `hasCursorLayer` - likewise passed straight through to `CursorPanel` (`cursorLyr !== null`), which uses it to suppress that annotation on a recording that carries a captured cursor layer.
- `hasWebcam` (panel pass, 2026-09-15) - `hasWebcamSignal(layout)` from `Editor`, passed straight through to `CameraPanel` for its no-webcam empty state; the same flag `ClassicShell` already hands the `Timeline` for the camera lane's hint.
- `addZoom` / `addSpotlight` / `addCameraMove: () => void` - add-at-playhead callbacks, passed to the Effects panel's quick-add buttons (and `addCameraMove` also to `CameraPanel`).

**Gone in M1a:** `sel` / `setSel` / `dur` / `aimMode` / `onAimMode` / `onSeek` / `layoutPresets` / `arrangeOn` / `onArrange`. All nine existed only for the inspector branch and now reach the inspectors through `PropertiesSlot`.
