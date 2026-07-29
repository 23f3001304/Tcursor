# src/editor/EditorPanels.tsx

The left-hand inspector/panel router: shows an inspector for the current timeline selection (zoom, effect, layout segment, or camera-move keyframe), else the panel for the active rail tab. Split out of `Editor` (which was over the file's line limit) - it owns only the "which panel" switch; every edit still flows through the `applyOp`/`saveDocSettings` callbacks it's handed.

## EditorPanels

```tsx
export function EditorPanels({
  doc, sel, tab, dur, setSel, setTab, timeMs, running, aiError, aiLog, onRun, applyOp, saveDocSettings,
  moveMode, requestMoveMode, camDraftRef, addZoom, addSpotlight, addCameraMove,
}: {
  doc: EditDoc;
  sel: string | null;
  tab: Tab;
  dur: number;
  setSel: Dispatch<SetStateAction<string | null>>;
  setTab: Dispatch<SetStateAction<Tab>>;
  timeMs: number;
  running: boolean;
  aiError: string | null;
  aiLog: string[];
  onRun: () => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  moveMode: boolean;
  requestMoveMode: (want: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
}): JSX.Element
```

Renders exactly one left-panel slot: an inspector when something is selected, otherwise the panel for the active rail `tab`.

### Props

- `doc: EditDoc` - the current edit document; selection lookups (`doc.zooms.find(...)`, etc.) and every panel's settings read from it.
- `sel: string | null` / `setSel` - the selected timeline item's id (a zoom, effect, layout segment, or camera-move keyframe), or `null` when nothing is selected.
- `tab: Tab` / `setTab` - the active rail tab (from `src/editor/shell/Rail.tsx`), shown when `sel` is `null`.
- `dur: number` - clip duration, passed to every inspector for range clamping.
- `timeMs: number` - playhead position; used by the Effects panel's "Add layout" button (drops a segment at the playhead) and passed to `CameraPanel`.
- `running` / `aiError` / `aiLog` / `onRun` - AI Director state and the run trigger, passed straight through to `AiPanel`.
- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - the shared mutation entry point; every inspector and panel button applies an `EditOp` through this.
- `saveDocSettings: (s: EditDoc["settings"]) => void` - bulk-saves a patched `settings` object; used by panels that edit `Settings` fields directly (background, cursor, camera, captions, audio) rather than emitting an `EditOp`.
- `moveMode` / `requestMoveMode` / `camDraftRef` - webcam "Move mode" state, threaded to `CameraPanel` so it can toggle drag-to-reposition and read the live unsaved pose (`CamPose` from `src/editor/stage/cameraMoves.ts`).
- `addZoom` / `addSpotlight` / `addCameraMove: () => void` - add-at-playhead callbacks, passed to the Effects panel's quick-add buttons (and `addCameraMove` also to `CameraPanel`).

### Behavior

**Selection derivation.** `selZoom`/`selEffect`/`selLayout`/`selCamMove` look up `sel` against `doc.zooms`/`doc.effects`/`doc.layout`/`doc.camera_moves`. The first one found (checked in that order) wins and renders its inspector; if none match, the router falls through to the `tab` switch.

**Inspector priority.** `ZoomInspector` > `EffectInspector` > `LayoutInspector` > `CameraMoveInspector` > the tab panel. Each inspector gets `onApply={applyOp}` and `onClose={() => setSel(null)}` (deselecting returns to the tab panel underneath). `EffectInspector` additionally gets `onDimCamera`, which patches `doc.settings.clickfx.spotlight_dim_camera` via `saveDocSettings`.

**Tab panels.** `"ai"` renders `AiPanel` (model comes from `doc.settings.ai_model`); `"background"`/`"cursor"`/`"camera"`/`"captions"`/`"audio"`/`"effects"` render their matching panel, each wired to patch its own settings slice via `saveDocSettings` and to close back to `"ai"`. Any other tab value renders a generic capitalized-title stub ("`{tab}` settings land here next.") as a placeholder for panels not yet built. `CameraPanel` additionally receives `doc`, `timeMs`, `applyOp`, and the move-mode props (it both reads/writes `camera_moves` via ops and `appearance` via settings). `AudioPanel` reads/writes `audio_offset_ms`, `audio_mic_volume`, and `audio_sys_volume` as three separate `saveDocSettings` calls. `EffectsPanel` wires `onAddLayout` to an inline `add_layout_seg` at the rounded playhead (`layout: "camera"`, `dur_ms: 2000`), alongside the passed-through `addZoom`/`addSpotlight`/`addCameraMove`.

**Transition.** The whole slot is wrapped in `AnimatePresence mode="popLayout"` keyed on `sel ?? tab`, so switching between inspectors/panels (or deselecting) slides the new content in from the left (`x: -8 -> 0`) while the old one exits, rather than popping instantly.

### Used by

`Editor` (`src/editor/Editor.tsx`) - renders it inside `e-body`, between `Rail` and `Stage`, passing all editor-owned state and callbacks straight through.
