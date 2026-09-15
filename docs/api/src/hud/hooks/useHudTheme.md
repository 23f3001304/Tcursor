# src/hud/hooks/useHudTheme.ts

Loads the persisted theme and accent on mount, keeps System mode reactive to the OS for the rest of the session, and hands back the one callback `Preferences` needs to change either. Split out of `Hud.tsx`: it is a lifecycle of its own, with its own ref, and nothing else in the HUD reads it.

## useHudTheme

```ts
export function useHudTheme(): (ui: InterfaceSettings) => void;
```

### Returns

The `onUiChange` callback `Hud` passes to `Preferences`. Calling it stores `{ theme, accent }` in the hook's ref and applies them at once, so a change is visible before it is persisted and the OS listener below keeps working against the new values.

### Behavior

One effect, mounted once:

1. `getSettings()`, then `applyTheme(s.ui.theme, s.ui.accent)`.
2. A `matchMedia("prefers-color-scheme: dark")` listener that re-applies on an OS change, so **System** stays reactive for the whole session.

The listener reads `{ theme, accent }` out of a ref, not out of state. Closing over state would mean the handler applied whatever the theme was when the effect ran, forever - the stale-closure bug this shape exists to avoid. The listener is removed on unmount.

`ui.animated_brand` (Task 39's feel knob) is deliberately not read here: the idle bar's mark is always at rest and the take pill has no mark, its recording tell being the clock's breathing dot. The editor's top-bar mark still honours the knob.

### Used by

- `src/hud/Hud.tsx`.
