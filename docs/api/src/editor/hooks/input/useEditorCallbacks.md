# src/editor/hooks/input/useEditorCallbacks.ts

Stabilizes the fistful of `Editor`-level callbacks handed down as props to the memoized hot tree (`Stage`/`Transport`/`Timeline`/`EditorPanels`). Split back out of `Editor.tsx`, reversing the 2026-09-15 filing pass's merge: that merge was measured before the one-time prettier run expanded these eight `useCallback`s, and eight user-gesture handlers are a responsibility of their own rather than a tail on a component. See `Editor.md`'s "Render hygiene" for the whole picture this is one piece of.

## useEditorCallbacks

```ts
export function useEditorCallbacks(args: {
  applyOp: (op: EditOp) => Promise<EditDoc | null>;
  sel: string | null;
  dur: number;
  timeMsRef: RefObject<number>;
  trimRangeRef: RefObject<{ inMs: number; outMs: number }>;
  setTimeMs: (ms: number) => void;
  setPlaying: (fn: boolean | ((p: boolean) => boolean)) => void;
  setAimOn: (v: boolean) => void;
  setQuality: (fn: (q: number) => number) => void;
  setMuted: (fn: (m: boolean) => boolean) => void;
  requestMoveMode: (want: boolean) => void;
}): {
  onTime: (ms: number) => void;
  aimAt: (x: number, y: number) => void;
  onMoveMode: (want: boolean) => void;
  onSeek: (ms: number) => void;
  onPlayToggle: () => void;
  onAspect: (aspect: Aspect) => void;
  cycleQuality: () => void;
  onMuteToggle: () => void;
}
```

### Why `applyOp` is a PARAMETER, not built here

`applyOp` is built by `useEditorSession` because several OTHER hooks need it BEFORE this one can run (`useMoveModeGuard(doc, applyOp)` for `requestMoveMode`, which this hook in turn needs for `onMoveMode`) - building it in here too would create a circular dependency between the two hooks. Everything this hook returns is instead built FROM the already-stable `applyOp` it's handed.

### Why `timeMs`/`trimRange` are refs, not plain values

The playhead changes every tick (`~16-60/sec` during playback or a scrub), but none of these eight
callbacks actually need to CLOSE OVER its live value at build time - they only need to read
whatever it currently is at the moment they're CALLED (a user gesture, never a tick). Taking
`timeMsRef`/`trimRangeRef` (both reassigned every render by `Editor.tsx`, same pattern as its own
`docRef`) instead of `timeMs: number`/`trimRange: {...}` means none of these `useCallback`s need
the ticking value in their dependency array, so their identity survives a tick - which is the
entire point: `onSeek`/`onPlayToggle` are props of `Transport` (`React.memo`'d), and an unstable
prop there would re-render it on every tick regardless of everything else being stabilized.


### Returns

- `onTime(ms)` - clamps playback to the trim's out point (parks `timeMs` exactly on `outMs`
  rather than the frame or two of overshoot the video reports before pausing lands), else just
  `setTimeMs(ms)`. Passed to `Stage` as its `onTime` prop.
- `aimAt(x, y)` - `update_zoom { target: { fixed: { x, y } } }` on the CURRENTLY selected zoom
  (read from a `sel`-mirroring ref, so this doesn't need to change identity every time the
  selection itself changes for an unrelated reason). Passed to `Stage` as `onAimAt`.
- `onMoveMode(want)` - clears `aimOn` before delegating to `requestMoveMode` (the two on-stage
  gestures are mutually exclusive - see `Editor.md`'s "Zoom aiming"). Passed to `EditorPanels` as
  `requestMoveMode`.
- `onSeek(ms)` - `setPlaying(false); setTimeMs(ms)`. Shared by `Transport` (skip-to-start/end) and
  `Timeline` (track-body scrub/drop) - previously two identical inline arrows, one per caller.
- `onPlayToggle()` - the Play/Pause button's handler: flips `playing`, and if starting playback
  from outside the resolved trim range, snaps `timeMs` to `trimRange.inMs` first (via the refs)
  so play never starts inside a dimmed, trimmed-out region. Passed to `Transport` as `onPlay`.
- `onAspect(aspect)` - `set_aspect` through `applyOp`. Passed to `Transport` as `onAspect`.
- `cycleQuality()` - cycles the preview proxy height `480 -> 720 -> 1080 -> 480`. Passed to
  `Transport` as `onQuality`.
- `onMuteToggle()` - flips `muted`. Passed to `Transport` as `onMute`.


### Used by

`Editor` (`src/editor/Editor.tsx`) - the sole caller.
