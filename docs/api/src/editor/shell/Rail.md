# src/editor/shell/Rail.tsx

Back in its classic place by the owner's call (2026-09-13): the area/workspace shell that briefly replaced it is archived on `archive/m1a-shell`. `Tab` and the tab list come from `PanelTabs.tsx`.

Left-side icon rail for the editor: one icon button per entry in the ordered `PANEL_TABS` manifest (eight of them since Layouts joined on 2026-09-14). The "ai" tab keeps the violet accent whether or not it is active; the active tab additionally carries a shared, layout-animated pill. Stateless beyond receiving the current tab and a setter.

## Rail

```tsx
export function Rail({ tab, onTab }: { tab: Tab | null; onTab: (t: Tab) => void }): JSX.Element
```

Renders the vertical icon rail with one button per `Tab`.

### Props

- `tab: Tab | null` - the currently active tab, or `null` while the panel column is collapsed. *Why:* determines which button gets the `.on` class (and so the `.e-ric-hl` pill and `aria-current`). With `null` no button is marked and no pill is drawn - nothing is current, because no panel is showing.
- `onTab: (t: Tab) => void` - called with the tab ID when the user clicks a button. *Why:* tab selection state is lifted to `Editor` so the panel area can respond.

### The rail is the toggle, but not the rule (2026-09-14)

The panel column collapses, and the rail is how it does: pressing the tab that is already open closes it, pressing any other opens on that one, and pressing anything while collapsed opens again. **The rail itself does not know that** - it reports which icon was pressed and nothing more. The decision is `nextTab` (`panelState.ts`), which `ClassicShell` wraps into the `onTab` it passes here, so the rail and the panels' own close X (which calls `setTab(null)`) cannot drift apart.

The rail never collapses with the column: it is the way back in, and a rail that vanished with the panel would leave the editor with no visible way to reopen one.

### Behavior

**Button rendering.** `PANEL_TABS` (`PanelTabs.tsx`) is the single source of truth for order, icon, and label; `PANEL_TABS.map` produces one `motion.button` per entry (the app-wide press spring, scale .96). The active button gets `.e-ric.on` and a shared `motion.span.e-ric-hl` (`layoutId: "rail-active"`) that glides between tools; `id === "ai"` always gets `.ai`, so the AI icon keeps its brand color even at rest.

**Tooltips and hover (look pass, 2026-09-14).** The owner's note was that the rail's icons needed both. Each button is wrapped in `Tooltip` (`controls/surfaces/Tooltip.tsx`), which fades its label in to the right of the rail after a 350ms hover dwell - and the plain `title` attribute stays on the button underneath it, so the native tooltip is still there as the fallback and for anything reading the accessibility tree. Hover itself now lands the icon on `--e-raised` at full `--e-fg` (`.e-ric:not(.on):hover`, `stage/stage.css`), which is where the rail's rules moved with the rest of the centre column's. The `:not(.on)` guard is deliberate: the active button already sits on its own pill, and on the AI tab that pill is violet-tinted, so a grey fill under it would only muddy the color.

`aria-current` marks the active button (the ARIA pattern for "this nav item points at where you already are"), not `aria-pressed`. It is written as `tab === id || undefined`, so a collapsed rail (`tab === null`) marks nothing at all.

### Notes

- The rail has no local state and no effects; `Tooltip` owns the one timer involved.
- `PANEL_TABS` is module-level and constant; adding a new tab requires only a new entry in that array.
- The `.ai` class is applied based on `id === "ai"`, not `tab === "ai"`, so it persists even when another tab is selected - the AI button retains its brand color at rest.
