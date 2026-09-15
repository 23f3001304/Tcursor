# src/editor/EditorPanels.tsx

The `panel` editor type's body: one panel per tab, and nothing else. Originally the left-hand inspector/panel router (an inspector for the current timeline selection, else the panel for the active rail tab); M1a removed the selection half. The four inspectors are the `properties` editor type now (`shell/PropertiesSlot.tsx`), so selecting a pill on the timeline no longer replaces whatever panel this area is showing. Panels and selection are separate axes.

**Collapsing (2026-09-14).** `tab` here is a NON-null `Tab`: whether a panel is showing at all is `ClassicShell`'s business, and it unmounts this whole column (on a width animation) when the rail is collapsed - see `ClassicShell.md`. What changed inside this file is the close action. Every panel's `onClose` was `setTab("ai")`, which routed to the AI Director rather than closing anything; they all now share one `collapse = () => setTab(null)`, so the X does what an X says it does. `setTab`'s type widened to `Dispatch<SetStateAction<Tab | null>>` to carry it.

## EditorPanels

`folder` is threaded through for one panel: `BackgroundPanel` imports the user's own background INTO the recording's folder (`background/<file>`), so it needs to know which one.

```tsx
export const EditorPanels: React.MemoExoticComponent<(props: EditorPanelsProps) => JSX.Element>
```

Renders exactly one panel: the one for `tab`. `React.memo`'d (render hygiene pass) - see "Render hygiene" below. Every prop is documented in `editorPanelsProps.md`.

### Behavior

**Tab panels.** `"ai"` renders `AiPanel` (model comes from `doc.settings.ai_model`); `"background"`/`"cursor"`/`"camera"`/`"layouts"`/`"hotkeys"`/`"audio"`/`"effects"` render their matching panel, each wired to patch its own settings slice via `saveDocSettings`. Every one of those gets `onClose={collapse}` for its `PanelHeader` - the shared `() => setTab(null)`, which collapses the column rather than routing to the AI Director the way `setTab("ai")` used to. `CameraPanel` additionally receives `doc`, `timeMs` (the gated one - see Props), `applyOp`, and the move-mode props; it now reads `appearance` without writing it (its `onChange` prop is gone - see `CameraPanel.md`). `LayoutsPanel` receives `doc`, `timeMsRef` and `saveDocSettings`, and reaches the app config itself over IPC for its saved looks. `AudioPanel` reads/writes `audio_offset_ms`, `audio_mic_volume`, and `audio_sys_volume` as three separate `saveDocSettings` calls. `EffectsPanel` wires `onAddLayout` to an inline `add_layout_seg` at `Math.round(timeMsRef.current)` (`layout: "camera"`, `dur_ms: 2000`), alongside the passed-through `addZoom`/`addSpotlight`/`addCameraMove`.

**`"captions"` (M5 T1, then T6, 2026-09-15).** The tab that used to be `"captions"` was actually the hotkey-chord overlay's settings; it was relabeled `"hotkeys"` (now rendering the renamed `HotkeysPanel`, unchanged in behavior) and a REAL `"captions"` tab took its old slot. T1 left an inline placeholder there; T6 replaced it with `panels/captions/CaptionsPanel.tsx` - the transcribe card, the style controls and the transcript. It is the only panel that takes `sel` / `onSel` / `onSeek` (its transcript rows select and seek, the two things clicking a timeline pill does) and the only one that takes `reloadDoc` (transcription writes `edit.json` on the backend, so the doc is re-read rather than patched from an op).

**Exhaustiveness (Task 26).** `Tab` (`src/editor/shell/PanelTabs.tsx`) is a closed 9-member union and the if/else chain above covers all 9 explicitly, so the final `else` is unreachable in practice - it used to render a dead "`{tab}` settings land here next." placeholder stub. That branch now calls `assertNever(tab)` instead: since `tab`'s type only narrows to `never` there if every real member was already handled, adding a new `Tab` variant without a matching branch here is a **compile-time** type error (`tab` fails to narrow to `never`) rather than a silent runtime fallback to a stub that could never actually render. Adding `"layouts"` (and later `"captions"`) is what exercised that guarantee: the union grew and this file failed to typecheck until its branch existed.

**Transition.** The whole slot is wrapped in `AnimatePresence mode="popLayout"` keyed on `tab` alone (it was `sel ?? tab` before M1a), so switching panels slides the new one in from the left (`x: -8 -> 0`) while the old one exits, rather than popping instantly. A selection change no longer keys this at all, which is exactly what "a panel tab change and a selection are independent" means in practice. This is the tab-to-tab swap only; opening and closing the column itself is a width animation one level up, on `.e-panel-wrap` in `ClassicShell` - two animations, because they are two different things happening and blending them into one would mean a panel that re-typesets while the column slides.

### Render hygiene

`React.memo`'d - this component (and whichever single panel it's currently showing) used to re-render on EVERY `Editor` render, including a pure playhead tick during playback, regardless of which panel was actually visible. For memo to actually hold, `Editor.tsx` passes every callback prop here (`applyOp`, `onRun`, `requestMoveMode`, `addZoom`/`addSpotlight`/`addCameraMove`) already `useCallback`'d/stable, and `EditorSlot` gates `timeMs` (see Props) rather than passing the live ticking value unconditionally. The `SlotProps` bundle in between is rebuilt every render, but it is spread back into individual props here, so memo still compares the values it always did.

### Used by

`ClassicShell` (`src/editor/shell/ClassicShell.tsx`) - inside the `.e-panel-wrap` column, which exists only while `tab !== null`.

**Classic layout (2026-09-13).** The selection branch is `shell/PropertiesSlot.tsx`, shown on the RIGHT of the stage by `ClassicShell` while something is selected; this router shows the rail's tab on the left, and the two are independent.
