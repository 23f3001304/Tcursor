# src/hud/components/StateSwap.tsx

The HUD's two bar states - the idle bar and the take pill (`TakeBar`) - swapped with a frost-and-spring interstate (owner, 2026-09-14: "frosting blur then deblur animations and some gooey spring transforms" between states).

## StateSwap

```ts
export function StateSwap({ take, onSettled, idle, pill }: {
  take: boolean; onSettled: (take: boolean) => void; idle: ReactNode; pill: ReactNode;
}): JSX.Element
```

### Props

- `take: boolean` - which state the flow is in (`recording || saving` in `Hud`). Drives the `AnimatePresence` key, so flipping it starts the swap.
- `onSettled: (take: boolean) => void` - fires in the gap between the leaving state finishing its frost-out and the arriving one mounting, with the state that is about to show. `Hud` feeds it to `takeShown`, which is what `useHudWindowSize` sizes the window for - so the window glide starts at the same moment the pill (or the bar) starts arriving, and the two land together.
- `idle`, `pill` - the two states' markup. Only one is mounted at a time.

### Behavior

- `AnimatePresence mode="wait"`, `initial={false}`: the leaving state frosts over first (opacity to 0, `blur(14px)`, scale 0.92; 140ms, ease-in), then the arriving one comes out of the same frost. No interstate on first mount.
- The arriving scale is a spring (stiffness 340, damping 15) that overshoots a hair - the "gooey" landing - while blur and opacity are plain tweens (260ms and 200ms), since a spring on a blur reads as flicker rather than bounce.
- The wrapper is `.swap` (`hud.css`): a plain column, so the idle titlebar and row stack exactly as they did without it. The `.hud` surface itself rounds from the bar's 14px corners into the pill's 31px over a CSS transition at the same time (`.hud` `transition: border-radius`), since `Hud` flips `as-take` when `takeShown` flips.

### Used by

- `src/hud/Hud.tsx` - `<StateSwap take={take} onSettled={setTakeShown} pill={<TakeBar .../>} idle={<IdleCard .../>} />`, the only child of `.hud`: nothing hides the bar any more, since Settings and Preferences are body states of the idle card (2026-09-14).

## FROST

```ts
export const FROST = { shown: SHOWN, frosted: FROSTED, arrive: ARRIVE, leave: LEAVE }
```

The interstate's own four constants, exported so a surface that swaps in elsewhere under the same HUD uses exactly these numbers rather than a second, drifting copy of them.

- `shown` / `frosted` - the two Motion states (`opacity`/`filter`/`scale`).
- `arrive` - the spring-on-scale, tween-on-blur transition into `shown`.
- `leave` - the fast ease-in out to `frosted`.

### Used by

- `src/hud/components/SourcesSheet.tsx` - the sheet frosts in on mount with `frosted`/`shown`/`arrive`, and its rows/target-list flip cross-frosts with all four.
