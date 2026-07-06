# src/hud/components/morph.ts

Animates the Tauri window between two logical sizes via `requestAnimationFrame` with an ease-out-cubic curve, keeping the top-left corner fixed. Used when the HUD transitions between the compact bar and the settings box so the resize glides instead of snapping.

## morphWindow

```ts
export function morphWindow(fromW: number, fromH: number, toW: number, toH: number, ms: number): Promise<void>
```

Smoothly resizes the current Tauri window from `(fromW, fromH)` to `(toW, toH)` over `ms` milliseconds.

### Inputs

- `fromW: number` / `fromH: number` - starting logical size in pixels. *Why explicit:* the tween needs a stable start value so it produces a smooth animation even when called mid-transition.
- `toW: number` / `toH: number` - target logical size in pixels. *Why separate from from:* the caller knows final dimensions from layout constants in `Hud.tsx` (`WIDTH`, `BOX_W`, `BOX_H`).
- `ms: number` - animation duration. *Why a parameter:* lets callers vary speed; `Hud.tsx` uses 200 ms for all transitions.

### Returns

`Promise<void>` that resolves when the animation completes, or immediately in reduced-motion / zero-duration mode. Callers can chain work after the resize finishes, e.g. `morphWindow(...).then(() => setBarShown(true))`.

### Behavior

- Checks `matchMedia("(prefers-reduced-motion: reduce)")` at call time. If the query matches, or if `ms <= 0`, the function calls `win.setSize(new LogicalSize(toW, toH))` once and returns a resolved promise - no animation loop runs.
- Otherwise, the `rAF` loop computes `k = easeOutCubic(clamp(elapsed / ms, 0, 1))` and calls `win.setSize(new LogicalSize(round(fromW + (toW - fromW) * k), round(fromH + (toH - fromH) * k)))` each frame.
- The loop exits when `elapsed / ms >= 1`, then the promise resolves.
- Uses `getCurrentWindow()` from `@tauri-apps/api/window`; must be called from a Tauri webview context.

### Used by

- `src/hud/Hud.tsx` - `openPanel()` morphs from bar size to box size; `restoreBar()` morphs back after the settings box closes
