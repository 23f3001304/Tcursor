# src/editor/shell/Rail.tsx

Left-side icon rail for the editor. Defines the `Tab` union type and the ordered `TABS` manifest, then renders one icon button per tab. The "ai" tab receives the violet accent class when active; all others use the standard active class. Stateless beyond receiving the current tab and a setter.

## Tab

```ts
export type Tab = "ai" | "background" | "cursor" | "camera" | "captions" | "audio" | "effects"
```

Discriminated string union enumerating every editor panel. Used by `Editor` to track `tab` state, by `Rail` to determine the active button, and by the panel-rendering branch in `Editor` to decide what to display.

## Rail

```tsx
export function Rail({ tab, onTab }: { tab: Tab; onTab: (t: Tab) => void }): JSX.Element
```

Renders the vertical icon rail with one button per `Tab`.

### Props

- `tab: Tab` - the currently active tab. *Why:* determines which button gets the `.on` class (and the `.ai` accent class when `tab === "ai"`).
- `onTab: (t: Tab) => void` - called with the tab ID when the user clicks a button. *Why:* tab selection state is lifted to `Editor` so the panel area can respond.

### Behavior

**Button rendering.**
The local `TABS` array is the single source of truth for order, icon, and label. Each entry is `{ id: Tab; icon: ComponentType<{ size?: number }>; label: string }`. `TABS.map` produces one `motion.button` per entry (design/premium-pass D6: the app-wide press spring, scale .96); no conditional rendering. The active button gets `.e-ric.on`; additionally, when `id === "ai"` the button gets `.e-ric.ai` regardless of whether it is active, so the AI icon always carries its violet accent color.

**Tooltip.**
Each button's `title` attribute is the full label (e.g. "AI Director", "Background") for native tooltip display.

### Notes

- The rail has no local state and no effects.
- TABS is module-level and constant; adding a new tab requires only a new entry in that array.
- The `.ai` class is applied based on `id === "ai"`, not `tab === "ai"`, so it persists even when another tab is selected -- the AI button retains its brand color at rest.
