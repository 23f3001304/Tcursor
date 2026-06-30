# src/editor/Transport.tsx

Thin transport bar rendered between the Stage and the Timeline. Provides add-layer and split-at-playhead tool buttons (currently stubs), skip-to-start / play-pause / skip-to-end playback controls, a formatted time readout, and mute / fit-to-window buttons (currently stubs). Stateless: all controls call back into `Editor`.

## Transport

```tsx
export function Transport({ timeMs, dur, playing, onPlay, onSeek }: {
  timeMs: number; dur: number; playing: boolean; onPlay: () => void; onSeek: (ms: number) => void;
}): JSX.Element
```

Renders the transport bar with playback controls and the current time / total duration display.

### Props

- `timeMs: number` - current playhead position in milliseconds. *Why:* displayed in the time readout as `fmt(timeMs)`.
- `dur: number` - total clip duration in milliseconds. *Why:* displayed as the denominator in the time readout (`fmt(dur)`), and passed to `onSeek` by the skip-to-end button.
- `playing: boolean` - whether playback is active. *Why:* toggles the play/pause icon and the button title.
- `onPlay: () => void` - called when the play/pause button is clicked. *Why:* playback state is owned by `Editor`; Transport only toggles it.
- `onSeek: (ms: number) => void` - called with `0` (skip to start) or `dur` (skip to end). *Why:* seek state is owned by `Editor`.

### Behavior

**Playback controls.**
- Skip-to-start calls `onSeek(0)`.
- Play/Pause button calls `onPlay()` and shows `IconPlayerPause` when `playing`, `IconPlayerPlay` otherwise. The button title text also switches.
- Skip-to-end calls `onSeek(dur)`.

**Time readout.**
`fmt(timeMs) + " / " + fmt(dur)` in the `.e-time` span. Both are in `M:SS` format.

**Stub buttons.**
Add-layer (IconPlus), Split (IconScissors), Mute (IconVolume), and Fit-to-window (IconArrowsMaximize) are rendered but have no `onClick` handlers. They represent planned future actions and occupy their layout position now.

**Layout.**
The two tool buttons sit on the left. A flex spacer (`.e-sp`) separates them from the centered playback cluster. Another spacer separates the time readout from the right-side utility buttons.

### Notes

- Transport has no local state and no effects. Its rendered output is a pure function of its props.
- Stub buttons are not `disabled` -- they have no handler but are visually interactive. Disabling them would require explaining why, which is deferred to when they are implemented.
