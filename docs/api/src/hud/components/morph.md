# src/hud/components/morph.ts

Animates the Tauri window between two logical sizes via `requestAnimationFrame` with an ease-out-cubic curve, either keeping the top-left corner fixed or keeping the window's horizontal centre (the bar<->take-pill glide, the only caller left since Settings became a sheet inside the card), so a resize glides instead of snapping.

## MorphAnchor

```ts
export type MorphAnchor = "corner" | "centre"
```

Which point of the window stays put while it changes size. `"corner"` is Tauri's own `setSize` behaviour (top-left fixed), and the default. `"centre"` keeps the window's horizontal midpoint, so the bar shrinking into the take pill (and back) stays where the eye was instead of collapsing toward its left end.

## keepHidden

```ts
export const keepHidden: () => Promise<void>
```

Re-asserts that the HUD window is excluded from screen capture (`setCapturable(false)`, the `set_capturable` command). Windows has been seen to drop the exclude-from-capture affinity across a resize: on 2026-09-14 the take pill showed up in a display capture while the idle card, which had not been resized since launch, never did. So every `morphWindow` and every snap resize in `useHudWindowSize` ends here. Idempotent and one IPC; a rejection is swallowed because the backend already logs the read-back mismatch (`capture_exclusion.rs`). Safe around the hand-off to the editor: `useEditorData` re-asserts the opposite on the editor's own mount, so a morph that ends after the hand-off cannot leave the editor hidden from screenshots.

## morphWindow

```ts
export async function morphWindow(fromW: number, fromH: number, toW: number, toH: number, ms: number, anchor?: MorphAnchor): Promise<void>
```

Smoothly resizes the current Tauri window from `(fromW, fromH)` to `(toW, toH)` over `ms` milliseconds.

### Inputs

- `fromW: number` / `fromH: number` - starting logical size in pixels. *Why explicit:* the tween needs a stable start value so it produces a smooth animation even when called mid-transition, and with `"centre"` the shift each frame is measured from `fromW`.
- `toW: number` / `toH: number` - target logical size in pixels. *Why separate from from:* the caller knows final dimensions from layout constants (`useHudWindowSize.ts`'s `WIDTH`/`IDLE_HEIGHT` and `TAKE_WIDTH`/`TAKE_HEIGHT`).
- `ms: number` - animation duration. `useHudWindowSize` uses 220ms for the pill.
- `anchor?: MorphAnchor` - `"corner"` by default.

### Returns

`Promise<void>` that resolves when the animation completes, or after the single snap in reduced-motion / zero-duration mode. Callers can chain work after the resize finishes, or `await` it, as `applySize` does before returning.

### Behavior

- With `"centre"`, reads `win.outerPosition()` and `win.scaleFactor()` once, before the first frame; each frame then also calls `win.setPosition` with the start x plus half the width change so far, converted to physical pixels (`(fromW - w) * scale / 2`), and the start y. Kept, not re-centred on the screen: the window stays wherever the user put it.
- Checks `matchMedia("(prefers-reduced-motion: reduce)")` at call time. If the query matches, or if `ms <= 0`, the function sets the final size (and, for `"centre"`, the final position) once - no animation loop runs.
- Otherwise, the `rAF` loop computes `k = easeOutCubic(clamp(elapsed / ms, 0, 1))` and calls `win.setSize(new LogicalSize(round(fromW + (toW - fromW) * k), round(fromH + (toH - fromH) * k)))` each frame, then the position for that width.
- The loop exits when `elapsed / ms >= 1`, then `keepHidden()` runs (the snap branch runs it too) and the promise resolves.
- Uses `getCurrentWindow()` from `@tauri-apps/api/window`; must be called from a Tauri webview context.

### Used by

- `src/hud/hooks/useHudWindowSize.ts` - `applySize` glides between the idle bar and the take pill (and over the Sources sheet opening) with `"centre"`. The only caller: `Hud.tsx`'s bar-to-settings-box morph was removed on 2026-09-14, when Settings and Preferences became sheets inside the idle card.
