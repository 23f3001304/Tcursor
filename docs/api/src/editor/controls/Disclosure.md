# src/editor/controls/Disclosure.tsx

The usability pass's one escape valve for a panel whose content cannot fit its slot: **one quiet row per panel**, holding that panel's least-used controls.

**One, not several.** A panel of collapsed sections is a filing cabinet, and the point of the pass was to stop making people hunt. The rule is enforced by convention rather than by code: each panel renders at most one `Disclosure`, always last, always labelled "More".

**When a panel is allowed to use one.** Only when the arithmetic says so. Every panel was measured by rows against a 620px slot; the four that still did not fit after the strips and the two-up rows landed are the four that carry a disclosure (Background, Cursor, Camera, Effects). The other three (AI Director, Audio, Captions) fit and have none.

## readDisclosure

```ts
export function readDisclosure(id: string): boolean
```

Is this panel's disclosure remembered as open? Reads `tcursor.panel.more.<id>`.

Storage can throw outright (a webview with site data blocked), not merely return `null`, and a panel must still render, so every failure reads as **closed**. Closed is also the safe default for a brand-new user: it is the state the height budget was measured against.

### Behaviors

- `only ever reports open for the exact stored flag`.
- `keys panels apart, so one panel's More does not open another's`.

## writeDisclosure

```ts
export function writeDisclosure(id: string, open: boolean): void
```

Remember this panel's state. Silent on failure, for the same reason: a panel whose disclosure forgets is a smaller problem than a panel that throws while you are toggling it.

## Disclosure

```tsx
export function Disclosure({ id, label = "More", children }: {
  id: string;
  label?: string;
  children: ReactNode;
}): JSX.Element
```

### Props

- `id` - the storage key suffix, one per panel (`"background"`, `"cursor"`, `"camera"`, `"effects"`). Panels remember apart, so opening Effects' More does not open Background's.

### What it looks like, and what it deliberately is not

Not a card: no plane, no stroke, no fill, and no rule above it. A dim 11px word at section-heading weight plus a chevron, so a closed panel reads as **finished** rather than truncated. That follows the panel sheet's "at most one card level" rule and benchmark tell (e)2 (bordered boxes stacked inside bordered panels).

### Accessibility

The row is a `<button>` with `aria-expanded` and `aria-controls` pointing at the body (`useId`). While closed the children are **not in the tree at all**, so nothing inside is in the tab order and nothing inside is announced. That is `AnimatePresence` doing the unmount rather than a `display: none`, which is what keeps a closed disclosure genuinely free.

### Motion

One tween, the m1a Look paragraph's content-swap timing: 0.16s, `[0.4, 0, 0.2, 1]`. The body animates `height: 0 <-> auto` with `opacity`, the chevron rotates 0 to 90 degrees, and that is the whole vocabulary. Under `useReducedMotion` the same tween runs at `duration: 0` rather than being dropped, so `AnimatePresence` keeps owning the unmount either way and the open and closed states are identical to the animated ones.

### Styling

`.e-more` / `.e-more-btn` / `.e-more-chev` / `.e-more-body` / `.e-more-inner` in `controls/controls.css`. The body's top gap lives on `.e-more-inner`: animating the outer box's height while its own padding changes makes the content jump at both ends of the tween.

### Used by

- `src/editor/panels/BackgroundPanel.tsx` (`id="background"`) - Look, Frame and Accent Colors.
- `src/editor/panels/CursorPanel.tsx` (`id="cursor"`) - Motion and Click.
- `src/editor/panels/CameraPanel.tsx` (`id="camera"`) - the ring, and the two margin nudges.
- `src/editor/panels/EffectsPanel.tsx` (`id="effects"`) - the always-on Spotlight, and the video effect.
