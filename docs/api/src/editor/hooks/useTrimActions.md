# src/editor/hooks/useTrimActions.ts

The three trim actions the transport's In / Out / reset buttons fire. Kept out of `Editor` so the "trim to playhead" rules live in one place (and `Editor` stays under the line limit).

## useTrimActions

```ts
export function useTrimActions(
  doc: EditDoc | null,
  timeMsRef: RefObject<number>,
  dur: number,
  applyOp: (op: EditOp) => Promise<EditDoc | null>,
): { onTrimIn: () => void; onTrimOut: () => void; onResetTrim: () => void }
```

### Inputs

- `doc: EditDoc | null` - read for `doc.trim`; defaults to `{ in_ms: 0, out_ms: 0 }` (untrimmed) while the doc hasn't loaded yet.
- `timeMsRef: RefObject<number>` - the current playhead position, as a ref (not a plain `timeMs: number` - render hygiene pass); each trim action snaps the corresponding trim point to `Math.round(timeMsRef.current)`, read at CALL time rather than closed over at build time.
- `dur: number` - the clip's real duration, passed to `resolveTrim` to get the effective (clamped) trim range used by the guard checks below.
- `applyOp: (op: EditOp) => Promise<EditDoc | null>` - applies the resulting `set_trim` op.

### Returns

`useCallback`'d (render hygiene pass - deps are `dur`/the resolved trim's own `inMs`/`outMs`/`applyOp`, none of which change on a playhead tick since `timeMs` is read from `timeMsRef` instead of being a dependency), so all three stay referentially stable across a tick - `Transport` (`React.memo`'d) needs that to actually skip re-rendering for them.

- `onTrimIn: () => void` - sets `trim.in_ms` to the rounded playhead, preserving `trim.out_ms` (`0` still means "no out trim"). No-ops if the playhead is at or after the resolved out point, so the range can never collapse to zero length or invert.
- `onTrimOut: () => void` - sets `trim.out_ms` to the rounded playhead, preserving `trim.in_ms`. A playhead at or past the clip's real duration clears the out trim back to `0` ("runs to the true end", matching `TrimOverlay`'s convention and `resolveTrim`'s handling of `0`) rather than writing the exact duration. No-ops if the playhead is at or before the resolved in point.
- `onResetTrim: () => void` - applies `set_trim` with `{ in_ms: 0, out_ms: 0 }`, clearing both trim points back to "untrimmed".

### Used by

`Editor` (`src/editor/Editor.tsx`) - `onTrimIn`/`onTrimOut`/`onResetTrim` are passed straight through to `Transport`'s In/Out/reset buttons.
