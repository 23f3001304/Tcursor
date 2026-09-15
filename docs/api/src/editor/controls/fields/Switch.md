# src/editor/controls/fields/Switch.tsx

An iOS-style toggle switch used across every settings panel.

## Switch

```tsx
export function Switch({ on, onChange, ariaLabel, disabled, title }: { on: boolean; onChange: (v: boolean) => void;
  ariaLabel?: string; disabled?: boolean; title?: string }): JSX.Element
```

A `role="switch"` button (`aria-checked={on}`) with a `layout`-animated thumb (`motion.span`, 0.16s tween). `onChange` is called with `!on` on click.

**Paint lives in CSS now (panel pass).** The track was an inline `background`/`border` pair, which meant a stylesheet hover rule could never win against it - so `.sw` (`controls/controls.css`) owns the whole track: `--e-raised` when off, one step lighter on hover, and **`--e-primary` when on**. This is where the panels spend their one accent: the benchmark's rule is one accent used only for interactive/selected state, and an engaged switch is exactly that. The border is gone entirely (surfaces read by lightness, not strokes). Only the thumb's animated `left` is still inline, because Motion animates it.

The thumb is `--e-fg` off and white on, on a 40x24 track - no separate "muted dot" state is needed once the track itself carries the colour. (The track was 40x23 until the usability pass put a 24px floor under every clickable control; the thumb is absolutely positioned from its flex-centred static position, so it re-centred on its own and `Switch.tsx` did not change.)

### Props

- `disabled?: boolean` / `title?: string` (T34 L3) - a switch whose OFF state the BACKEND would reject has to read as unavailable rather than silently snap back. Introduced for `LayoutInspector`'s panel-visibility rows: `set_arrangement` rejects a write that would hide both panels, and a rejected op still resolves, so a live-but-ignored switch would look like it worked and leave a phantom undo step behind. `disabled` sets the native attribute (so the button is genuinely inert and exposed as such) and dims the track to 0.45; `title` carries the reason on hover.
- `useReducedMotion` (panel pass) drops the `layout` animation, so the thumb jumps rather than slides. Nothing else about the control changes.
- `ariaLabel?: string` (Task 11) - sets `aria-label` on the button. The panel pass added it at every panel call site that had been relying on adjacent text alone. `Switch` renders a custom `role="switch"`, not a native `<input>`, so wrapping it in a `<label>` does NOT auto-associate a name the way it would for a real form control - most call sites instead rely on adjacent visible text for context, but `EffectInspector`'s `OverrideField` (whose visible switchrow text is the generic "Override", not the field name - see `EffectInspector.md`'s "One name, once") passes this explicitly (`` `Override ${label}` ``) so the control still has an unambiguous accessible name.
