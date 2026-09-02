# src/hud/components/RecordingControls.tsx

The recording row's right-hand cluster - timer, "Paused" chip, Pause/Resume, and the hero Record/Stop/Saving button. Split out of `Hud.tsx` (200-line cap) alongside `CamTile`/`RecMeter`; no behavior changed in the move.

## RecordingControls

```ts
export function RecordingControls({ recording, paused, elapsed, saving, exporting, toggle, togglePause }: {
  recording: boolean; paused: boolean; elapsed: number; saving: boolean; exporting: boolean;
  toggle: () => void; togglePause: () => void;
}): JSX.Element
```

### Props

- `recording: boolean` / `paused: boolean` - `Hud`'s `useRecordingFlow` state. Gates the timer, "Paused" chip, and Pause/Resume button - none of the three render while `!recording`.
- `elapsed: number` - `Hud`'s `useRecordingTimer(recording, paused)` value, formatted via `formatTimer`.
- `saving: boolean` - `Hud`'s `useRecordingFlow().saving`. Disables the hero button and switches its label to "Saving…".
- `exporting: boolean` - `Hud`'s local export-progress flag. Also disables the hero button.
- `toggle: () => void` / `togglePause: () => void` - `Hud`'s `useRecordingFlow().toggle`/`togglePause`, wired straight to the hero button and the Pause/Resume button respectively.

### Behavior

Owns no state - purely a rendering of the truth its props already carry:
- Timer (`recording &&`): `<span className="timer">`, dimmed via a `.paused` class while `paused`.
- "Paused" chip (`recording && paused &&`): a `motion.span` that opacity-pulses (`[0.55, 1, 0.55]` over 1.6s, looping).
- Pause/Resume button (`recording &&`): label reads `paused ? "Resume" : "Pause"`.
- Hero button: renders UNCONDITIONALLY (the only piece spanning all three bar states) - `disabled={exporting || saving}`, label `saving ? "Saving…" : recording ? "Stop" : "Record"`, a `motion.button` with its own `whileTap` spring (`scale: 0.92`, stiffness 500/damping 30) rather than `hud.css`'s plain `.btn:active` CSS scale (explicitly excluded there via `.btn:not(.rec):active` so the two don't fight over the same `transform`).

### Used by

- `src/hud/Hud.tsx` - `<RecordingControls recording={recording} paused={paused} elapsed={elapsed} saving={saving} exporting={exporting} toggle={toggle} togglePause={togglePause} />`, the last child of the recording row's `.row`, after `.spacer`.
