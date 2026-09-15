# src/editor/shell/settings/EditorSettingsDialog.tsx

Task 35's project settings dialog, opened from `TopBar`'s gear button. Reuses the `ExportDialog`/`ShortcutsOverlay` modal pattern (`.e-modal-scrim`/`.e-modal`, 0.16s tween, Esc or scrim closes) and composes four per-section components (`ZoomDefaultsSection`, `MotionSection`, `ScreenSection`, `InterfaceSection`) plus a shortcuts row and a static "recording settings live in the recorder" note. It draws the line the whole feature is scoped to: settings that meaningfully change THIS project's render or the editor experience and had no editor surface before - not record-time settings (hotkeys/devices/game mode), which stay in the HUD.

## EditorSettingsDialog

```tsx
export function EditorSettingsDialog({ open, settings, onClose, onSaveSettings, onOpenShortcuts }: {
  open: boolean;
  settings: EditDoc["settings"];
  onClose: () => void;
  onSaveSettings: (next: EditDoc["settings"]) => void;
  onOpenShortcuts: () => void;
  onApplyToAll: () => void;
}): JSX.Element
```

### Props

- `open: boolean` - whether the dialog is visible. Owned by `Editor` (`showSettings`), same shape as `showExportDialog`/`showShortcuts`.
- `settings: EditDoc["settings"]` - the current doc settings (`doc.settings`), passed whole (not pre-sliced) since the three sections each need a different top-level slice (`zoom`, `appearance.screen`, `ui`).
- `onClose: () => void` - dismiss. Wired to the scrim's `onPointerDown`, the header's X button, and an Escape keydown listener (mirrors `ShortcutsOverlay`'s own Escape handling - `ConfirmDialog`/`ExportDialog` rely on the scrim/X only, but this dialog's spec calls for Esc explicitly).
- `onSaveSettings: (next: EditDoc["settings"]) => void` - the caller's `saveDocSettings`. Every section's `onChange` composes its own slice into a full `EditDoc["settings"]` (`setZoom`/`setScreen`/`setUi`, same `{ ...settings, key: value }` shape `EditorPanels.tsx` already uses for `CameraPanel`/`CursorPanel`/etc.) and calls this - so every write here is undoable and rev-bumping, identical to every other settings-editing panel.
- `onApplyToAll: () => void` - Motion's "Apply to all regions". *Why its own prop rather than going through `onSaveSettings`:* it is a DOC edit (`{ op: "apply_motion_default" }` through the caller's `applyOp`), not a settings write, so it takes a different path all the way up - `Editor` passes it as `EditorDialogs`' `onApplyMotion`.
- `onOpenShortcuts: () => void` - called by the Shortcuts row's View button. *Why the caller owns switching dialogs, not this component:* `ShortcutsOverlay` is a sibling modal owned by `Editor`, not a child of this one; `Editor`'s wiring (`() => { setShowSettings(false); setShowShortcuts(true); }`) closes this dialog first so the two scrims never stack.

### Behavior

**Sections, in spec order.** `ZoomDefaultsSection` (`settings.zoom`), `MotionSection` (`settings.motion`, plus `settings.zoom.camera_smoothing_ms` as a scalar and `onApplyToAll`), `ScreenSection` (`settings.appearance.screen`), `InterfaceSection` (`settings.ui`) - each an `.e-sec` block with its own title + per-section reset icon (`.e-secrow`), not one dialog-wide reset. See each section's own doc for its fields/reset literal.

**Shortcuts row.** A single `.e-sec` with a "Keyboard shortcuts" label and a `.e-modal-btn` "View" button (`IconKeyboard`) calling `onOpenShortcuts` - reuses the existing T15 `ShortcutsOverlay` rather than duplicating its list.

**Recording settings note.** A final `.e-sec` with one dim `.e-lede` line ("Capture settings (hotkeys, devices, game mode) live in the recorder. They apply at record time.") and no controls - the explicit boundary from the HUD's own record-time settings surface.

**Sizing.** `.e-settings-modal` (420px, `max-height: min(78vh, 640px)`, `overflow-y: auto`) - wider and scrollable unlike the smaller fixed-height `ConfirmDialog`/`ShortcutsOverlay` modals, since three sections' worth of controls can exceed a short window's height.

### Used by

- `Editor` (`src/editor/Editor.tsx`) - rendered near `ShortcutsOverlay`; `TopBar`'s gear button (`onOpenSettings`) sets `showSettings`, `settings={doc.settings}`, `onSaveSettings={saveDocSettings}`.

**`setUi` also mirrors to the app (2026-09-15).** Besides saving the project's `ui`, it calls `mirrorUiToApp` (`applyUi.md`) so the HUD and the next launch show the same theme and accent; `Editor`'s effect applies the change to the document root immediately. The owner had reported that changing the theme or the accent in Project Settings did nothing visible.
