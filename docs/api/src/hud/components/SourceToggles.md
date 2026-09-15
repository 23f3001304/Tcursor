# src/hud/components/SourceToggles.tsx

The idle card's four source switches as one labelled segmented row: camera, microphone, system audio, and the compatibility encoder. Split out of `IdleCard.tsx`, which is the card's frame and its three body states; this is the one row inside it that is four of the same thing.

## Toggles

```ts
export interface Toggles { camOn: boolean; micOn: boolean; sysOn: boolean; gameMode: boolean }
```

The four source switches, by `Hud`'s own state names. `onToggle` reports the key that was pressed; `Hud` flips the matching setter through a table keyed by this type, so adding a fifth toggle is one field and one button rather than a new handler.

## SourceToggles

```tsx
export function SourceToggles({ toggles, onToggle }: {
  toggles: Toggles;
  onToggle: (key: keyof Toggles) => void;
}): JSX.Element
```

### Props

- `toggles: Toggles` - the four states, `Hud`'s own. This component owns none of them.
- `onToggle: (key: keyof Toggles) => void` - which one was pressed. Not a new value: the caller already has the old one, and reporting the key keeps the four buttons identical.

### Behavior

**Icon-only**, four equal segments in one plane (`.tog-row`, `role="group"`, labelled "Sources"). The name is the tooltip and the `aria-label`, `aria-pressed` carries the state, and ON is the accent tint. They carried a word beside the icon until 2026-09-15, which the owner read as noise.

Three of the four swap their glyph with their state (camera/camera-off, mic/mic-off, speaker/speaker-off). The compatibility encoder keeps one glyph and says which way it is in its title, because there is no "CPU off" - the choice is which encoder, not whether to encode.

`TOGGLE_PRESS` is the shared press feel: `whileTap` scale 0.96 on a stiff spring, so all four answer identically.

### Used by

- `src/hud/components/IdleCard.tsx` - between the mic row and Record.
