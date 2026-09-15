# src/editor/shell/PanelTabs.tsx

The nine panel tabs: the union, the manifest, and the id list one caller needs. All of it used to live in `shell/Rail.tsx`; M1a moved it here when the rail became a workspace switcher, and it stayed after the classic rail came back.

## Tab

```ts
export type Tab = "ai" | "background" | "cursor" | "camera" | "layouts" | "captions" | "hotkeys" | "audio" | "effects"
```

The closed union naming every panel. `Editor` tracks it as `Tab | null` state (`null` = the panel column is collapsed - see `panelState.md`), `EditorPanels` routes on the non-null value with an exhaustive `assertNever` fallback, and `Rail` marks the active one. Adding a tab is one entry in `PANEL_TABS` plus one branch in `EditorPanels`; the `assertNever` makes forgetting the second half a compile error.

`"layouts"` joined on 2026-09-14, placed right after `"camera"` because the two are read together: what the webcam does over time, and what it (and the screen) look like in each layout.

`"captions"` and `"hotkeys"` (M5 T1, 2026-09-15): the tab that used to be `"captions"` was actually the hotkey-chord overlay's settings, so it was relabeled `"hotkeys"` and a real `"captions"` tab was added in its place, right before it. `EditorPanels`' `"captions"` branch is a placeholder for now (transcription lands in a later task); `"hotkeys"` renders the renamed `HotkeysPanel`, unchanged in behavior from the old `CaptionsPanel`.

## PANEL_TABS

```ts
export const PANEL_TABS: { id: Tab; icon: ComponentType<{ size?: number }>; label: string }[]
```

The single source of truth for order, icon and label, unchanged in shape from the old rail's `TABS`. `Rail.tsx` draws these nine tabs.

## TAB_IDS

```ts
export const TAB_IDS: readonly Tab[]
```

Every tab id, derived from `PANEL_TABS` so it cannot fall behind it. It exists for exactly one caller: `readPanelTab` (`panelState.ts`) has to decide whether a string it read out of `localStorage` still names a tab in THIS build, and takes the list as an argument rather than importing it, so the storage helpers stay ignorant of what the tabs are.
