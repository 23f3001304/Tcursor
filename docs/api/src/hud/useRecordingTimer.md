# src/hud/useRecordingTimer.ts

Hook that tracks elapsed recording time in milliseconds, supporting pause and resume without drift. Uses two refs to accumulate time across multiple run segments so the displayed value is always accurate regardless of how many pause/resume cycles have occurred.

## useRecordingTimer

```ts
export function useRecordingTimer(active: boolean, paused: boolean): number
```

Returns elapsed recording time in milliseconds, updated approximately every 200 ms while active and not paused.

### Arguments

- `active: boolean` - master on/off. When `false`, resets the timer to 0 and clears the accumulator. When it becomes `true`, starts a new timing session from 0. *Why not derived from a recording state machine:* the hook owns only the timer concern; the parent decides when recording is active.
- `paused: boolean` - when `true` while `active` is also `true`, the interval does not run and the accumulated value is preserved frozen. *Why two separate flags rather than a three-state enum:* simpler to destructure from the parent's recording state.

### Returns

`number` - elapsed milliseconds. Starts at 0, ticks forward every 200 ms while running, freezes on pause, and resets to 0 when `active` becomes `false`.

### Behavior

The effect re-runs whenever `active` or `paused` changes:

- If `!active`: sets `elapsed` to 0, resets `acc.current` to 0, returns (no interval started).
- If `paused`: returns without starting an interval, leaving the current `elapsed` value frozen.
- Otherwise (active and not paused): captures `segStart.current = Date.now()`, then starts a 200 ms `setInterval`. Each tick sets `elapsed = acc.current + (Date.now() - segStart.current)`, so the displayed value is always the total of all completed segments plus the current one.

On effect cleanup (triggered by a dependency change or unmount): adds the current segment's duration (`Date.now() - segStart.current`) to `acc.current` and clears the interval. This ensures that on the next run, the newly started segment continues from the correct accumulated total.

Internal refs (not exposed):
- `acc` (`useRef<number>`) - milliseconds accumulated across all completed segments before the current one.
- `segStart` (`useRef<number>`) - `Date.now()` captured when the current segment began.
