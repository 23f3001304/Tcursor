# src/hud/components/formatTimer.ts

Pure formatting utility that converts a millisecond count to a `M:SS` display string. Used by the recording timer badge in the HUD bar.

## formatTimer

```ts
export function formatTimer(ms: number): string
```

Converts elapsed milliseconds to a `M:SS` string for display.

### Inputs

- `ms: number` - elapsed time in milliseconds, non-negative. *Why milliseconds:* `useRecordingTimer` accumulates time in ms for sub-second precision during pause/resume; the display only needs whole seconds, so the conversion happens here rather than in the hook.

### Returns

`string` in `M:SS` format where seconds are always zero-padded to two digits and minutes are never padded. No upper bound is enforced; hours roll into the minutes field naturally (e.g. `3_600_000` returns `"60:00"`).

### Behaviors (pinned by unit tests in `src/hud/components/formatTimer.test.ts`)

- `formatTimer(0)` returns `"0:00"`.
- `formatTimer(5_000)` returns `"0:05"`.
- `formatTimer(65_000)` returns `"1:05"`.
- `formatTimer(600_000)` returns `"10:00"`.

All four cases are in the `"formats milliseconds as M:SS"` test.
