# src/editor/EditorPanels.tsx

The left-hand inspector/panel router: shows an inspector for the current timeline selection (zoom, effect, layout segment, or camera-move keyframe), else the panel for the active rail tab. Split out of `Editor` (which was over the file's line limit) - it owns only the "which panel" switch; every edit still flows through the `applyOp`/`saveDocSettings` callbacks it's handed.

## EditorPanels

```tsx
export const EditorPanels: React.MemoExoticComponent<(props: {
  doc: EditDoc;
  sel: string | null;
  tab: Tab;
  dur: number;
  setSel: Dispatch<SetStateAction<string | null>>;
  setTab: Dispatch<SetStateAction<Tab>>;
  timeMs: number;
  timeMsRef: RefObject<number>;
  running: boolean;
  exporting: boolean;
  aiError: string | null;
  aiLog: string[];
  aiProgress: { step: number; total: number } | null;
  onRun: () => void;
  onAutoModel: (v: string) => void;
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  saveDocSettings: (s: EditDoc["settings"]) => void;
  moveMode: boolean;
  requestMoveMode: (want: boolean) => void;
  camDraftRef: RefObject<CamPose | null>;
  osCursorInVideo: boolean;
  aimMode: boolean;
  onAimMode: (on: boolean) => void;
  onSeek: (ms: number) => void;
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
}) => JSX.Element>
```

Renders exactly one left-panel slot: an inspector when something is selected, otherwise the panel for the active rail `tab`. `React.memo`'d (render hygiene pass) - see "Render hygiene" below.

### Props

- `doc: EditDoc` - the current edit document; selection lookups (`doc.zooms.find(...)`, etc.) and every panel's settings read from it.
- `sel: string | null` / `setSel` - the selected timeline item's id (a zoom, effect, layout segment, or camera-move keyframe), or `null` when nothing is selected.
- `tab: Tab` / `setTab` - the active rail tab (from `src/editor/shell/Rail.tsx`), shown when `sel` is `null`.
- `dur: number` - clip duration, passed to every inspector for range clamping.
- `timeMs: number` - playhead position, passed to `CameraPanel`. **Gated by the caller** (render hygiene pass): `Editor.tsx` only passes the real, ticking value while `tab === "camera"` - every other tab/inspector doesn't read `timeMs` at all, so `Editor` passes a constant `0` instead, letting this component's `React.memo` actually skip a re-render on a tick while some other panel is showing.
- `timeMsRef: RefObject<number>` - the SAME playhead, but as a ref that's always current regardless of `tab` - used by the Effects panel's "Add layout" button (drops a segment at the playhead), which needs the TRUE current time at click time no matter which panel happens to be showing, without that need forcing `timeMs` itself to always be live (which would defeat the gating above).
- `running` / `aiError` / `aiLog` / `aiProgress` / `onRun` - AI Director state and the run trigger, passed straight through to `AiPanel`. `aiProgress` (`{step, total} | null`, from `Editor`'s `useDirector()`) is the live "k of N" for the choreographed reveal's progress line - `null` before a run starts and while the plan is still being fetched.
- `exporting: boolean` (bug-sweep-2 Task 8, L3) - passed straight through to `AiPanel`, which locks its run button on it too, matching `Transport`'s wand and its own play/trim/aspect `locked` gate: a director pass mutating `edit.json` while the exporter renders from its own doc snapshot would silently diverge the two.
- `onAutoModel: (v: string) => void` - the QUIET model-write (no undo step; see `Editor`'s `useDocSettings`), passed straight through to `AiPanel` for its mount-time auto-default-model effect. Distinct from the Engine picker's own `onChange`, which is built inline here from `saveDocSettings` (records an undo step, since that's a deliberate user pick).
- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - the shared mutation entry point; every inspector and panel button applies an `EditOp` through this.
- `saveDocSettings: (s: EditDoc["settings"]) => void` - bulk-saves a patched `settings` object; used by panels that edit `Settings` fields directly (background, cursor, camera, captions, audio) rather than emitting an `EditOp`.
- `moveMode` / `requestMoveMode` / `camDraftRef` - webcam "Move mode" state, threaded to `CameraPanel` so it can toggle drag-to-reposition and read the live unsaved pose (`CamPose` from `src/editor/stage/cameraMoves.ts`). `moveMode` also reaches `ZoomInspector`, which disables its "Aim on stage" button while Move mode owns the canvas pointer.
- `aimMode` / `onAimMode` - on-stage zoom-aiming state, threaded to `ZoomInspector`'s Aim toggle. Only meaningful for a Region-target zoom; see `Editor.md`.
- `onSeek: (ms: number) => void` - `Editor`'s playhead seek (the same path `Timeline`/`Transport` use), threaded to `ZoomInspector` so changing a zoom-scoped control (Target, webcam action) outside the zoom's span jumps the playhead into it - see `ZoomInspector.md`'s `zoomScopedSeekMs`.
- `osCursorInVideo` - passed straight through to `CursorPanel`, which annotates the "System" style when the recording has no baked OS cursor to show (see its own doc).
- `addZoom` / `addSpotlight` / `addCameraMove: () => void` - add-at-playhead callbacks, passed to the Effects panel's quick-add buttons (and `addCameraMove` also to `CameraPanel`).

### Behavior

**Selection derivation.** `selZoom`/`selEffect`/`selLayout`/`selCamMove` look up `sel` against `doc.zooms`/`doc.effects`/`doc.layout`/`doc.camera_moves`. The first one found (checked in that order) wins and renders its inspector; if none match, the router falls through to the `tab` switch.

**Inspector priority.** `ZoomInspector` > `EffectInspector` > `LayoutInspector` > `CameraMoveInspector` > the tab panel. Each inspector gets `onApply={applyOp}` and `onClose={() => setSel(null)}` (deselecting returns to the tab panel underneath). `EffectInspector` additionally gets `onDimCamera`, which patches `doc.settings.clickfx.spotlight_dim_camera` via `saveDocSettings`.

**Tab panels.** `"ai"` renders `AiPanel` (model comes from `doc.settings.ai_model`; as of Task 26 it also gets `onClose={() => setTab("ai")}` for its `PanelHeader` - a no-op on this particular tab, but keeps the header wiring uniform across every panel since `PanelHeader.onClose` isn't optional); `"background"`/`"cursor"`/`"camera"`/`"captions"`/`"audio"`/`"effects"` render their matching panel, each wired to patch its own settings slice via `saveDocSettings` and to close back to `"ai"`. `CameraPanel` additionally receives `doc`, `timeMs` (the gated one - see Props), `applyOp`, and the move-mode props (it both reads/writes `camera_moves` via ops and `appearance` via settings). `AudioPanel` reads/writes `audio_offset_ms`, `audio_mic_volume`, and `audio_sys_volume` as three separate `saveDocSettings` calls. `EffectsPanel` wires `onAddLayout` to an inline `add_layout_seg` at `Math.round(timeMsRef.current)` (`layout: "camera"`, `dur_ms: 2000`), alongside the passed-through `addZoom`/`addSpotlight`/`addCameraMove`.

**Exhaustiveness (Task 26).** `Tab` (`src/editor/shell/Rail.tsx`) is a closed 7-member union and the if/else chain above covers all 7 explicitly, so the final `else` is unreachable in practice - it used to render a dead "`{tab}` settings land here next." placeholder stub. That branch now calls `assertNever(tab)` instead: since `tab`'s type only narrows to `never` there if every real member was already handled, adding a new `Tab` variant without a matching branch here is a **compile-time** type error (`tab` fails to narrow to `never`) rather than a silent runtime fallback to a stub that could never actually render.

**Transition.** The whole slot is wrapped in `AnimatePresence mode="popLayout"` keyed on `sel ?? tab`, so switching between inspectors/panels (or deselecting) slides the new content in from the left (`x: -8 -> 0`) while the old one exits, rather than popping instantly.

### Render hygiene

`React.memo`'d - this component (and whichever single panel it's currently showing) used to re-render on EVERY `Editor` render, including a pure playhead tick during playback, regardless of which panel was actually visible. For memo to actually hold, `Editor.tsx` passes every callback prop here (`applyOp`, `onRun`, `requestMoveMode`, `addZoom`/`addSpotlight`/`addCameraMove`) already `useCallback`'d/stable, and gates `timeMs` (see Props) rather than passing the live ticking value unconditionally. `onAutoModel`/`saveDocSettings` (from `useDocSettings`) were already `useCallback`'d before this pass.

### Used by

`Editor` (`src/editor/Editor.tsx`) - renders it inside `e-body`, between `Rail` and `Stage`, passing all editor-owned state and callbacks straight through.
