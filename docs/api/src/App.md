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

**Render.**
A ternary -- no `AnimatePresence` here. `Editor` receives `folder` and `onClose`. `Hud` receives `onEdit`, which is `openEditor`. When `view.v` is `"hud"` the `Editor` is fully unmounted; its local state (playback position, selection, doc) is discarded when the user goes back to the HUD.

### Notes

- `getCurrentWindow()` is called once per render inside the component; for a single-window Tauri app this is cheap and avoids a hook.
- The window ops (`setSize`/`center`/`setCapturable`) are async but the `setView` call does not await their completion -- the resize is cosmetic and the new view must be immediately interactive. Each is wrapped in `safe()` so one failing op cannot block the view switch.
