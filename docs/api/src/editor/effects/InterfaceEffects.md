# src/editor/effects/InterfaceEffects.tsx

The editor's click ripple, and the component that owns the `interface_effects` setting for the whole feature.

**One overlay for the window, not a layer per control.** A thin ring blooms from 8px to 56px under every pointerdown in the editor chrome and is gone in 320ms. One element, one listener, transforms and opacity only, `pointer-events: none` - it can neither steal a click nor shift a layout, which is the entire design constraint.

## InterfaceEffects

```tsx
export function InterfaceEffects(): JSX.Element | null
```

### Props

None. Everything it needs it reads: the setting from `getSettings()`, the pointer from a window listener, the tint from the DOM.

### Where it mounts, and why it portals

Mounted from `src/App.tsx` **beside** `<Editor>` (see [App](../../App.md)), never from inside the editor tree - `Editor.tsx` and `shell/ClassicShell.tsx` belong to another agent's surface, and the feature should own no part of them. It is rendered only while the view is the editor; the HUD has its own motion language and no ripples.

But it **portals** its overlay into the live `.editor` element, for two reasons that both matter:

1. **Tokens.** The `--e-*` palette is declared on `.editor` and custom properties inherit only to descendants. A sibling of `.editor` would resolve `--e-line-strong` and `--e-primary` to nothing and paint an invisible ring.
2. **Z-order.** `.e-modal-scrim` is `z-index: 100` *inside* `.editor`'s own stacking context (`.editor` is `position: fixed; z-index: 100`). A **sibling** of `.editor` could not sit under that scrim at any z-index - it would either be under the whole editor or over its modals. As a **child** at `z-index: 99` the overlay clears the panels, the timeline and the director layers (all < 51) and still passes under a modal, which is what the brief asked for.

The portal target is resolved in an effect (`document.querySelector(".editor")`) after the first commit - `Editor` is a sibling in the same commit, so its root is in the DOM by then. It is re-queried whenever the flag flips back on, which covers the editor having been closed and reopened while the effects were off. Until it resolves, the component renders `null`; the pointer listener is a separate effect and does not wait for it.

### Reading the setting

`getSettings()` on mount, and again on every window `focus`. Preferences is a separate surface, so focus returning to this window is exactly when a flip over there becomes visible here - no event to subscribe to, no reload. The resolved value is published through [effectsFlag](effectsFlag.md) (which is how `useMagnetic`, three levels down someone else's tree, hears about it) as well as held in local state.

`s.ui.interface_effects !== false` rather than a plain read, so a backend that does not send the field yet resolves to **on** rather than to `undefined`. A rejected `getSettings()` leaves whatever the flag already says.

### The listener

One `pointerdown` on `window`, `{ capture: true, passive: true }`.

**Capture phase on purpose.** The timeline's pills and the stage's handles all `stopPropagation()` their own `pointerdown`; a bubble-phase listener would simply never hear those. Capture hears everything, and the opt-out attribute - not propagation - is what decides which surfaces stay silent. See [ripples](ripples.md) for `suppressesRipple` / `rippleTone` and the two surfaces that opt out.

Off unmounts the overlay and clears the ripple list, and the effect returns before registering anything: **zero listeners**, not a listener that early-returns.

### Motion

Each ripple is a `motion.span` drawn at its final 56px and animated `scale: RIPPLE_FROM / RIPPLE_TO -> 1` with `opacity: 0.9 -> 0` over `RIPPLE_MS`, easing `[0.2, 0, 0.2, 1]` (fast out of the pointer, long tail). It removes itself in `onAnimationComplete`.

`AnimatePresence` earns its place on the **cap**, not on the normal life cycle: a ripple that finishes its own fade is already invisible when it leaves, but the seventh ripple evicts the oldest mid-bloom, and without an `exit` that one would vanish as a hard pop.

**`prefers-reduced-motion`** keeps the ripple and drops the growth: it appears at full size (`scale: 1`) at a lower opacity and fades over the same 320ms. The feedback is the point; the bloom is the flourish.

### Styling

`.e-ui-fx` and `.e-ui-ripple` in `effects/effects.css`. The ring is `--e-line-strong` - the editor's own "this hairline is active" value, the one the hover and focus states of every bordered control already reach for, which is what keeps a ripple reading as part of the chrome rather than as a colour of its own. `.accent` swaps it for `--e-primary` and adds `--e-glow-primary`, used verbatim here because (unlike a focused button) this element owns its whole `box-shadow` and has nothing to clobber.

The **focus ring** is deliberately *not* in that sheet and *not* gated by this setting: it lives on `.editor :focus-visible` in `editor.css`, because a visible keyboard focus is not a flourish a user may switch off.
