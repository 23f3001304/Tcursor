# src/App.tsx

Top-level view controller for TCursor's Tauri webview. Owns the single discriminated-union `view` state that switches the window between the floating HUD recorder and the windowed post-record editor. The window resize/move between the two modes is driven here so neither child has to know about the Tauri window API.

## View

```ts
type View = { v: "hud" } | { v: "editor"; folder: string }
```

Internal union describing which screen is active. When `v` is `"editor"`, `folder` carries the project path passed down to `Editor`.

## App

```tsx
export function App(): JSX.Element
```

Renders either `<Hud>` or `<Editor>` based on `view`, and manages the window size transition between them.

### Props

None. `App` is the application root mounted by `main.tsx`.

### Behavior

**State.** `view` starts as `{ v: "hud" }`. It transitions to `{ v: "editor", folder }` in `openEditor` and back to `{ v: "hud" }` in `closeEditor`. No other state lives here; recording state belongs to `Hud` and edit state belongs to `Editor`.

**`openEditor(folder)`.**
Each step runs through `safe()` so one failure cannot abort the rest. It makes the window resizable (the HUD window is non-resizable), drops always-on-top, sets a `880x560` minimum size (the floor below which the editor layout breaks), `setSize`s the window to a comfortable **non-fullscreen** rectangle, `center()`s it, and finally `setCapturable(true)` so the editor opts back into screen capture before setting `view` to `{ v: "editor", folder }`. (`Editor` also calls `setCapturable(true)` on mount as a backstop.)

*Sizing math:* the target size is `min(1440, availWidth - 120) x min(900, availHeight - 120)` - capped so it is never fullscreen, with margin so it always fits. *Why `center()`:* Tauri's built-in centering works on this window (the HUD uses it), unlike the earlier multi-monitor math. *Why a resizable window:* the user can move it (top-bar drag region) and resize it (see [ResizeEdges](editor/ResizeEdges.md) for the resize grips), so it is a real window, not a takeover.

**`closeEditor()`.**
Reverses the changes through `safe()`: `setCapturable(false)`, restore always-on-top and non-resizable, clear the minimum size with `setMinSize(null)` (*why:* the `880x560` editor floor would otherwise block shrinking back), `setSize` to the 980x132 HUD bar and `center()` it, then switch `view` back to `{ v: "hud" }`.

**Startup (`useEffect` on `[]`, runs once).**
First applies the stored theme: `getSettings()` then `applyTheme(s.ui.theme, s.ui.accent)`, which stamps `data-theme` on the document root and sets `--accent`. `Hud` re-applies the theme on every settings change and OS theme flip, but a launch that goes straight to the editor never mounts the HUD, so without this line an "always light" or "always dark" preference would only take effect once the HUD had been shown. Failures are swallowed; the stylesheet's `prefers-color-scheme` fallback stands until the settings arrive.
Then the cold-start file association: calls `getLaunchProject()`; a non-null folder means this process was launched by double-clicking a `.tcursor` file, and calls `openEditor(folder)` to route straight to the editor instead of the HUD. A normal launch resolves `null` and nothing happens - the effect runs after the initial render, so the HUD still mounts first and is briefly visible before the switch on a `.tcursor` launch. Failures (rejected promise) are swallowed; there is nothing useful to show if this fails. *Warm-launch is not covered:* if TCursor is already running when another `.tcursor` is opened, the OS starts a second process rather than notifying this one - see the Rust `LaunchProject` doc comment.

**Render.**
A ternary -- no `AnimatePresence` here. `Editor` receives `folder` and `onClose`. `Hud` receives `onEdit`, which is `openEditor`. When `view.v` is `"hud"` the `Editor` is fully unmounted; its local state (playback position, selection, doc) is discarded when the user goes back to the HUD.

The editor branch renders a fragment: `<Editor>` **and** `<InterfaceEffects />` (`src/editor/effects/InterfaceEffects.tsx`), the editor's click-ripple overlay.

*Why it mounts here and not inside the editor.* The micro-interaction feature deliberately owns no part of the editor tree - `Editor.tsx` and `shell/ClassicShell.tsx` are another surface's files - so its one component hangs off the view switch instead, beside the thing it decorates. It is **portalled** into the live `.editor` element at runtime, for its `--e-*` palette tokens (custom properties inherit only to descendants) and for its z-order (it has to sit under `.e-modal-scrim`, which lives inside `.editor`'s own stacking context); see [InterfaceEffects](editor/effects/InterfaceEffects.md).

*Why only on the editor branch.* The HUD has its own motion language and no ripples, and rendering the overlay there would leave a window-wide `pointerdown` listener attached across every recording. Unmounting with the view is what guarantees it costs nothing while the app is doing its actual job.

### Notes

- `getCurrentWindow()` is called once per render inside the component; for a single-window Tauri app this is cheap and avoids a hook.
- Within `openEditor` itself, `setView` DOES wait for the whole resize/center/capturable sequence above (each step is `await`ed in order, `safe()`-wrapped so one failing op cannot block the rest) - it is the last line, run only once every prior step has settled.
- `Hud`'s own "Open Project" button reaches `openEditor` through the identical `onEdit` prop, not through `getLaunchProject` - the two entry points (double-click vs. in-app button) converge on the same `openEditor(folder)` call.
- **`onEdit`'s contract is promisified (fix round 1, item 2, controller ruling 2026-09-02).** `openEditor`'s own `async` return type already satisfied `Hud`'s `onEdit?: (folder: string) => Promise<void>` - no change needed here - but every CALLER of `onEdit` now matters: `useRecordingFlow.finish` (via its `handOff` helper, `docs/api/src/hud/hooks/useRecordingFlow.md`) awaits it before deciding whether to reset its own `saving` flag, so the HUD's window-shrink-back-to-idle `setSize` can never fire while `openEditor`'s own resize sequence is still in flight on the SAME window (the two used to race - whichever `setSize` call resolved last silently won, so the editor could open bar-sized). `Hud.tsx`'s "Open Project" button (`openExistingProject`) awaits it too, for the same reason (though that path was never actually racing anything - it just keeps the contract honest end to end).
