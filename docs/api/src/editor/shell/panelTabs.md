# src/editor/shell/panelTabs.tsx

The seven panel tabs: the union, the manifest, and the compact segmented control they render as. All three used to live in `shell/Rail.tsx`; M1a moved them here when the rail became a workspace switcher, so the strip now renders inside the `panel` editor's own area header.

## Tab

```ts
export type Tab = "ai" | "background" | "cursor" | "camera" | "captions" | "audio" | "effects"
```

The closed union naming every panel. `Editor` tracks it as state, `EditorPanels` routes on it with an exhaustive `assertNever` fallback, `PanelTabStrip` marks the active one, and `StageToolbar` names one to quick-open. Importers moved from `shell/Rail` to this file in the same change; nothing else about the type changed, so M1b's collapse of these seven into four editor types is still a single edit here.

## PANEL_TABS

```ts
export const PANEL_TABS: { id: Tab; icon: ComponentType<{ size?: number }>; label: string }[]
```

The single source of truth for order, icon and label, unchanged from the old rail's `TABS`. Adding a tab is one entry here plus one branch in `EditorPanels`.

The compact tab strip this file used to export went with the area shell; the classic `Rail.tsx` draws these seven tabs now.
