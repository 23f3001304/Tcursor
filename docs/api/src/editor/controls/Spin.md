# src/editor/controls/Spin.tsx

Minimal Motion-driven spinner used across the editor for loading and export-in-progress states. Animates a continuous 360-degree rotation via Motion's `animate` prop instead of a CSS `@keyframes` rule, keeping all animation under Motion's scheduler and honoring `MotionConfig reducedMotion="user"`.

## Spin

```tsx
export function Spin({ size = 18 }: { size?: number }): JSX.Element
```

Renders a continuously rotating `IconLoader2` icon.

### Props

- `size?: number` - pixel size passed to `IconLoader2` and used as the icon's width and height. Defaults to 18. *Why a prop:* callers need different sizes (15 in TopBar export button, 16 in AiPanel run button, 22 in Stage empty state, 16 in Stage corner overlay).

### Behavior

**Rotation animation.**
`motion.span` wraps `IconLoader2` and uses `animate={{ rotate: 360 }}` with `transition={{ repeat: Infinity, duration: 0.8, ease: "linear" }}`. The span has `display: "flex"` so the icon is centered without extra wrapper markup.

*Why Motion instead of CSS keyframes:* Motion's `animate` participates in `MotionConfig reducedMotion="user"`, so the spinner stops automatically when the OS has reduced motion enabled. A CSS `@keyframes` animation would bypass that setting.

### Notes

- No props beyond `size`; color is inherited from the surrounding text color via CSS `currentColor`.
- The `0.8s` duration is deliberately slightly faster than a typical 1s spinner to read as "active computation" rather than "idle wait".
