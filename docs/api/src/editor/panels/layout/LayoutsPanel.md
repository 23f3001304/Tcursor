# src/editor/panels/layout/LayoutsPanel.tsx

The Layouts rail panel (2026-09-14): **how each of the five layouts looks, and looks you can save**. It is the editor's answer to two gaps at once.

**Gap 1 - four of the five layouts had no editor.** `CameraPanel` edited `settings.appearance.screen` and nothing else, so a recording's Camera, Presenter, Screen-only and Camera-only appearance was reachable only from the HUD's own Appearance settings, which edits the GLOBAL config rather than this project. This panel edits all five, one at a time, against a schematic of the one being edited, and writes the project.

**Gap 2 - a look could not be reused.** Every knob wrote `edit.json` and stayed there, so the second recording started from scratch. A **preset** is a snapshot of ALL FIVE layouts under a name, stored in the app config (`Settings.layout_presets`), which is what makes a look reusable on a project recorded months later.

## LayoutsPanel

```tsx
export function LayoutsPanel({ doc, timeMsRef, onSaveSettings, onClose }: {
  doc: EditDoc;
  timeMsRef: RefObject<number>;
  onSaveSettings: (s: EditDoc["settings"]) => void;
  onClose: () => void;
}): JSX.Element
```

### Props

- `doc: EditDoc` - the project. Two things are read off it: `doc.settings.appearance` (the five layouts being edited) and `doc.layout` (the segments, for the opening pick).
- `timeMsRef: RefObject<number>` - the live playhead, as a **ref** rather than a ticking prop. Read exactly once, in the `useState` initializer, to choose which layout the panel opens on. *Why a ref:* re-picking on every tick would yank the panel out from under a user who is mid-edit, and a ticking `timeMs` prop would defeat `EditorPanels`' `React.memo` on every frame of playback for a panel that does not otherwise need the time.
- `onSaveSettings: (s: EditDoc["settings"]) => void` - `Editor`'s `saveDocSettings`. One call per change, so one undo step per change, and the `rev` bump re-resolves the stage and the timeline - which is why a knob move is visible on the preview immediately.
- `onClose: () => void` - collapses the panel column (see `EditorPanels.md`).

### Two stores, on purpose

| What | Where it lives | How it is written |
| --- | --- | --- |
| The five layouts' knobs | the PROJECT, `doc.settings.appearance` | `onSaveSettings` (undo step, live preview) |
| Saved looks | the APP, `Settings.layout_presets` | `getSettings` / `setSettings` (read-modify-write) |

Applying a look is the one place the two meet: it reads the app config and writes the project. Nothing here ever writes a preset INTO `edit.json` - a project carries the look it was given, never the library it came from.

The app config is held in one `app` state, seeded by a `getSettings()` in a mount effect and replaced by every write (`writeApp` sets the state and fires `setSettings`, optimistic, `.catch(() => {})` - the same idiom `SettingsPanel`/`Preferences` use). Until that first read lands `app` is `null` and every preset action is a no-op: a write built on a half-read config would drop every global field the panel does not know about. The mount effect guards `setApp` with an `alive` ref, so a panel closed before the read resolves does not set state after unmount.

### Flow

1. **Which layout** - a `Picker` over the five layouts (`MODES`, `appearanceFields.ts`). It was a `Segmented` at `columns={2}` until the panel pass of 2026-09-15: five labels in a 2-column grid left one orphan cell, and the schematic right under the control already shows which layout is picked, so the dropdown costs nothing in legibility and reads as the plain control the owner prefers. It defaults to `layoutAtPlayhead(doc.layout, timeMsRef.current)` (see `layoutPresets.md`) and **changing it writes nothing**: picking a layout chooses what is being edited, it does not apply anything.
2. **The schematic** - `LayoutMiniPreview` for the picked layout, so a knob's effect is legible before the eye reaches the stage.
3. **The knobs** - `LayoutKnobs`, grouped Screen then Camera. Each change is a whole-`ModeAppearance` patch merged into `appearance[mode]` and saved.
4. **Reset this layout** - a text button writing `resetLayout(appearance, mode)`: that ONE layout back to how it ships, the other four exactly as the user left them. Deliberately not `PanelHeader`'s reset icon, whose fixed "Reset to defaults" label would claim a scope this button does not have.
5. **Saved looks** - `LayoutPresetList`, with the write callbacks built from the pure helpers in `layoutPresets.ts`.

### Reaching the export

Nothing in this panel needs a render change. The export already resolves every layout through `AppearanceSettings::for_id` and `layout_for` / `overlay_for` (`src-tauri/src/settings/appearance.rs`) from the doc's own `settings.appearance`, so a knob saved here is in the exported frame by the same path the Camera panel's knobs always were. The only backend change the feature needed was the `layout_presets` field itself.

### Used by

- `src/editor/EditorPanels.tsx` - the `tab === "layouts"` panel.
