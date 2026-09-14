# src/editor/EditorPanels.tsx

The `panel` editor type's body: one panel per tab, and nothing else. Originally the left-hand inspector/panel router (an inspector for the current timeline selection, else the panel for the active rail tab); M1a removed the selection half. The four inspectors are the `properties` editor type now (`shell/PropertiesSlot.tsx`), so selecting a pill on the timeline no longer replaces whatever panel this area is showing. Panels and selection are separate axes.

**Collapsing (2026-09-14).** `tab` here is a NON-null `Tab`: whether a panel is showing at all is `ClassicShell`'s business, and it unmounts this whole column (on a width animation) when the rail is collapsed - see `ClassicShell.md`. What changed inside this file is the close action. Every panel's `onClose` was `setTab("ai")`, which routed to the AI Director rather than closing anything; they all now share one `collapse = () => setTab(null)`, so the X does what an X says it does. `setTab`'s type widened to `Dispatch<SetStateAction<Tab | null>>` to carry it.

## EditorPanels

`folder` is threaded through for one panel: `BackgroundPanel` imports the user's own background INTO the recording's folder (`background/<file>`), so it needs to know which one.

```tsx
export const EditorPanels: React.MemoExoticComponent<(props: {
  folder: string;
  doc: EditDoc;
  tab: Tab;
  setTab: Dispatch<SetStateAction<Tab | null>>;
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
  addZoom: () => void;
  addSpotlight: () => void;
  addCameraMove: () => void;
  osCursorInVideo: boolean;
  hasCursorLayer: boolean;
}) => JSX.Element>
```

Renders exactly one panel: the one for `tab`. `React.memo`'d (render hygiene pass) - see "Render hygiene" below.

### Props

- `doc: EditDoc` - the current edit document; every panel's settings read from it.
- `tab: Tab` / `setTab` - the panel to render, and the setter every close X uses to collapse the column (`setTab(null)`). `tab` is non-null here; `Editor` and `ClassicShell` hold the nullable one.
- `timeMs: number` - playhead position, passed to `CameraPanel`. **Gated by the caller** (render hygiene pass): `ClassicShell` only passes the real, ticking value while `tab === "camera"` - every other panel doesn't read `timeMs` at all, so it passes a constant `0` instead, letting this component's `React.memo` actually skip a re-render on a tick while some other panel is showing.
- `timeMsRef: RefObject<number>` - the SAME playhead, but as a ref that's always current regardless of `tab`. Two callers: the Effects panel's "Add layout" button (drops a segment at the playhead), which needs the TRUE current time at click time no matter which panel happens to be showing; and `LayoutsPanel`, which reads it ONCE on mount to open on the layout the playhead is inside. Neither need forces `timeMs` itself to always be live, which would defeat the gating above.
- `running` / `aiError` / `aiLog` / `aiProgress` / `onRun` - AI Director state and the run trigger, passed straight through to `AiPanel`. `aiProgress` (`{step, total} | null`, from `Editor`'s `useDirector()`) is the live "k of N" for the choreographed reveal's progress line - `null` before a run starts and while the plan is still being fetched.
- `exporting: boolean` (bug-sweep-2 Task 8, L3) - passed straight through to `AiPanel`, which locks its run button on it too, matching `Transport`'s wand and its own play/trim/aspect `locked` gate: a director pass mutating `edit.json` while the exporter renders from its own doc snapshot would silently diverge the two.
- `onAutoModel: (v: string) => void` - the QUIET model-write (no undo step; see `Editor`'s `useDocSettings`), passed straight through to `AiPanel` for its mount-time auto-default-model effect. Distinct from the Engine picker's own `onChange`, which is built inline here from `saveDocSettings` (records an undo step, since that's a deliberate user pick).
- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - the shared mutation entry point; every panel button applies an `EditOp` through this.
- `saveDocSettings: (s: EditDoc["settings"]) => void` - bulk-saves a patched `settings` object; used by panels that edit `Settings` fields directly (background, cursor, camera, captions, audio) rather than emitting an `EditOp`.
- `moveMode` / `requestMoveMode` / `camDraftRef` - webcam "Move mode" state, threaded to `CameraPanel` so it can toggle drag-to-reposition and read the live unsaved pose (`CamPose` from `src/editor/stage/cameraMoves.ts`). `moveMode` also reaches `ZoomInspector`, which now lives in `PropertiesSlot` and gets it from the same `SlotProps` bundle.
- `osCursorInVideo` - passed straight through to `CursorPanel`, which annotates the "System" style when the recording has no baked OS cursor to show (see its own doc).
- `hasCursorLayer` - likewise passed straight through to `CursorPanel` (`cursorLyr !== null`), which uses it to suppress that annotation on a recording that carries a captured cursor layer.
- `addZoom` / `addSpotlight` / `addCameraMove: () => void` - add-at-playhead callbacks, passed to the Effects panel's quick-add buttons (and `addCameraMove` also to `CameraPanel`).

**Gone in M1a:** `sel` / `setSel` / `dur` / `aimMode` / `onAimMode` / `onSeek` / `layoutPresets` / `arrangeOn` / `onArrange`. All nine existed only for the inspector branch and now reach the inspectors through `PropertiesSlot`.

### Behavior

**Tab panels.** `"ai"` renders `AiPanel` (model comes from `doc.settings.ai_model`); `"background"`/`"cursor"`/`"camera"`/`"layouts"`/`"captions"`/`"audio"`/`"effects"` render their matching panel, each wired to patch its own settings slice via `saveDocSettings`. Every one of the eight gets `onClose={collapse}` for its `PanelHeader` - the shared `() => setTab(null)`, which collapses the column rather than routing to the AI Director the way `setTab("ai")` used to. `CameraPanel` additionally receives `doc`, `timeMs` (the gated one - see Props), `applyOp`, and the move-mode props; it now reads `appearance` without writing it (its `onChange` prop is gone - see `CameraPanel.md`). `LayoutsPanel` receives `doc`, `timeMsRef` and `saveDocSettings`, and reaches the app config itself over IPC for its saved looks. `AudioPanel` reads/writes `audio_offset_ms`, `audio_mic_volume`, and `audio_sys_volume` as three separate `saveDocSettings` calls. `EffectsPanel` wires `onAddLayout` to an inline `add_layout_seg` at `Math.round(timeMsRef.current)` (`layout: "camera"`, `dur_ms: 2000`), alongside the passed-through `addZoom`/`addSpotlight`/`addCameraMove`.

**Exhaustiveness (Task 26).** `Tab` (`src/editor/shell/panelTabs.tsx`) is a closed 8-member union and the if/else chain above covers all 8 explicitly, so the final `else` is unreachable in practice - it used to render a dead "`{tab}` settings land here next." placeholder stub. That branch now calls `assertNever(tab)` instead: since `tab`'s type only narrows to `never` there if every real member was already handled, adding a new `Tab` variant without a matching branch here is a **compile-time** type error (`tab` fails to narrow to `never`) rather than a silent runtime fallback to a stub that could never actually render. Adding `"layouts"` is what exercised that guarantee: the union grew and this file failed to typecheck until its branch existed.

**Transition.** The whole slot is wrapped in `AnimatePresence mode="popLayout"` keyed on `tab` alone (it was `sel ?? tab` before M1a), so switching panels slides the new one in from the left (`x: -8 -> 0`) while the old one exits, rather than popping instantly. A selection change no longer keys this at all, which is exactly what "a panel tab change and a selection are independent" means in practice. This is the tab-to-tab swap only; opening and closing the column itself is a width animation one level up, on `.e-panel-wrap` in `ClassicShell` - two animations, because they are two different things happening and blending them into one would mean a panel that re-typesets while the column slides.

### Render hygiene

`React.memo`'d - this component (and whichever single panel it's currently showing) used to re-render on EVERY `Editor` render, including a pure playhead tick during playback, regardless of which panel was actually visible. For memo to actually hold, `Editor.tsx` passes every callback prop here (`applyOp`, `onRun`, `requestMoveMode`, `addZoom`/`addSpotlight`/`addCameraMove`) already `useCallback`'d/stable, and `EditorSlot` gates `timeMs` (see Props) rather than passing the live ticking value unconditionally. The `SlotProps` bundle in between is rebuilt every render, but it is spread back into individual props here, so memo still compares the values it always did.

### Used by

`ClassicShell` (`src/editor/shell/ClassicShell.tsx`) - inside the `.e-panel-wrap` column, which exists only while `tab !== null`.

**Classic layout (2026-09-13).** The selection branch is `shell/PropertiesSlot.tsx`, shown on the RIGHT of the stage by `ClassicShell` while something is selected; this router shows the rail's tab on the left, and the two are independent.
