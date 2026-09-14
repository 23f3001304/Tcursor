# src/hud/components/RecordButton.tsx

The idle card's hero: the one red button across its bottom, "Record". What is left of the old `RecordingControls` cluster now that the take has its own pill (`TakeBar.tsx`), which carries the timer, the Paused state and the Pause/Stop pair.

## RecordButton

```ts
export function RecordButton({ disabled, onClick, label }: { disabled: boolean; onClick: () => void; label?: string }): JSX.Element
```

### Props

- `disabled: boolean` - `Hud`'s `exporting` flag: a take cannot start while an export from the HUD is running.
- `onClick: () => void` - `useRecordingFlow`'s `toggle`, which starts the take (the same handler `TakeBar`'s Stop calls to end it).
- `label?: string` - `"Record"` unless given; the card passes `Exporting… {pct}%` while an export runs, so the progress shows in the button's own place while it is disabled.

### Behavior

A `motion.button` with the `.btn.rec` classes (accent fill, white dot, full width and 48px in the card) whose press is its own Motion spring (`scale: 0.97`, stiffness 500 / damping 30) rather than `hud.css`'s `.btn:active` scale - that rule excludes `.rec` so the two never fight over one `transform`.

### Used by

- `src/hud/components/IdleCard.tsx` - the last child of the card body.
