# src/editor/shell/settings/applyUi.ts

What a project's `settings.ui` does to the running app. Until 2026-09-15 the editor's Settings > Interface wrote `doc.settings.ui` and nothing else: the theme only picked the synthetic cursor's light or dark sprite in the render (`resolve_dark`) and the accent only coloured the effects, so a change looked like it did nothing. The owner reads both as the interface's theme and colour, so now a change applies to the document root at once and is mirrored into the app config, which the HUD applies on its next mount. The doc copy stays the render's source of truth.

## applyProjectUi

```ts
export function applyProjectUi(ui: InterfaceSettings): void
```

`applyTheme(ui.theme, ui.accent)`: stamps `data-theme` and `--accent` on the document root. `Editor` calls it in an effect keyed on `doc.settings.ui`, so it runs on mount (the project's look wins over whatever the HUD stamped) and after every settings save, undo or redo.

## AppSettingsIo

```ts
export interface AppSettingsIo { getSettings: () => Promise<Settings>; setSettings: (s: Settings) => Promise<void> }
```

The two IPC calls `mirrorUiToApp` needs, injectable so the test never touches Tauri.

## mirrorUiToApp

```ts
export function mirrorUiToApp(ui: InterfaceSettings, io?: AppSettingsIo): Promise<void>
```

Read-modify-write of the app config: only `ui.theme` and `ui.accent` are copied over, every other field (including `ui.animated_brand`, `ui.interface_effects`, `ui.ai_choreography`, which stay per project) survives. Any IPC failure is swallowed, since the dialog has already saved the project and a missing app write must not throw out of a click handler. Called from `EditorSettingsDialog`'s `setUi`, an explicit user action, never from the mount effect.

### Used by

- `src/editor/Editor.tsx` - the `applyProjectUi` effect.
- `src/editor/shell/settings/EditorSettingsDialog.tsx` - `setUi` mirrors to the app.
