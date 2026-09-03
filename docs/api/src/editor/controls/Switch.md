# src/editor/controls/Switch.tsx

An iOS-style toggle switch used across every settings panel.

## Switch

```tsx
export function Switch({ on, onChange, ariaLabel, disabled, title }: { on: boolean; onChange: (v: boolean) => void;
  ariaLabel?: string; disabled?: boolean; title?: string }): JSX.Element
```

A `role="switch"` button (`aria-checked={on}`), spring-animated thumb (`motion.span`, `layout`). `on` picks the track/thumb colors (`--e-fg` track + near-black thumb when on; `--e-soft` track + `--e-dim` thumb when off - the off state reads clearly muted rather than as a bright floating dot). `onChange` is called with `!on` on click.

### Props

- `disabled?: boolean` / `title?: string` (T34 L3) - a switch whose OFF state the BACKEND would reject has to read as unavailable rather than silently snap back. Introduced for `LayoutInspector`'s panel-visibility rows: `set_arrangement` rejects a write that would hide both panels, and a rejected op still resolves, so a live-but-ignored switch would look like it worked and leave a phantom undo step behind. `disabled` sets the native attribute (so the button is genuinely inert and exposed as such) and dims the track to 0.45; `title` carries the reason on hover.
- `ariaLabel?: string` (Task 11) - sets `aria-label` on the button. `Switch` renders a custom `role="switch"`, not a native `<input>`, so wrapping it in a `<label>` does NOT auto-associate a name the way it would for a real form control - most call sites instead rely on adjacent visible text for context, but `EffectInspector`'s `OverrideField` (whose visible switchrow text is the generic "Override", not the field name - see `EffectInspector.md`'s "One name, once") passes this explicitly (`` `Override ${label}` ``) so the control still has an unambiguous accessible name.
