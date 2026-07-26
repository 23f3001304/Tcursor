# src/editor/timeline/TrimOverlay.tsx

Renders on top of `Timeline.tsx`'s track stack: dims the trimmed-out head/tail and provides two draggable edge handles (in/out) that commit `SetTrim`. Drag mechanics mirror `useRegionDrag` (a local draft moves live; the IPC write only happens once, on pointerup) but are inlined here rather than reusing that hook, since there is no "region" (start+end pair with a layer) - just two independent 1D positions. Reads the same `resolveTrim` (`src/lib/edit.ts`) the export gate and the playhead clamp use, so the dimmed region always matches what will actually be cut.

## TrimOverlay

```tsx
export function TrimOverlay({ trim, dur, trackRef, onApply }: {
  trim: Trim; dur: number; trackRef: React.RefObject<HTMLDivElement | null>;
  onApply: (op: EditOp) => Promise<EditDoc | null>;
}): JSX.Element | null
```

Renders the two dim overlays + two drag handles, or `null` when `dur <= 0` (nothing to trim yet).

### Props

- `trim: Trim` - the doc's raw trim fields (`in_ms`/`out_ms`).
- `dur: number` - the clip's real duration (ms) - the same basis `Timeline`'s ruler/pills use, so percentages line up exactly.
- `trackRef: RefObject<HTMLDivElement | null>` - `Timeline`'s `.e-tlbody` ref; drag math divides pointer x by this element's `getBoundingClientRect()`.
- `onApply: (op: EditOp) => Promise<EditDoc | null>` - applies `set_trim` once, on drag release.

### Behavior

**Resolved range.** `resolveTrim(trim, dur)` gives `{ inMs, outMs }` - `out_ms === 0` (unset) reads as the whole clip, so nothing renders trimmed until a real `SetTrim` lands.

**Drag.** `drag` state holds `{ mode: "in" | "out", ms }` while a handle is held; a `pointermove` listener (attached only while `drag` is non-null, mirroring `useRegionDrag`'s lifecycle) recomputes `ms` from the pointer's fraction of `trackRef`'s width, clamped so the two handles never cross within `MIN_GAP_MS` (200ms) of each other. `pointerdown` on a handle calls `e.stopPropagation()` first (so `Timeline`'s own track-body seek handler doesn't also fire) and seeds `drag` from the handle's current live position.

**Commit.** On `pointerup`, applies `{ op: "set_trim", in_ms, out_ms }` with the OTHER field left at its current `trim.*` value; if the dragged-out `outMs` reaches `dur` exactly, `out_ms` is written as `0` (back to "whole clip" / unset) rather than the literal duration, so a doc that's never actually been trimmed at that edge stays serialized as the `Trim::default()`-shaped "unset" value.

**Rendering.** Two `.e-trimdim` divs (absolute, `top:0;bottom:0`, spanning `.e-tlbody`'s full height like the playhead) cover `[0, liveIn]` and `[liveOut, dur]` - only rendered when non-empty (`liveIn > 0` / `liveOut < dur`). Two `.e-trimhandle` divs (thin vertical bars, `cursor: ew-resize`) sit at `liveIn`/`liveOut`.

### Notes

- No local commit debouncing is needed beyond "only on pointerup" - unlike `useRegionDrag`'s callers, there is only ever one drag target class (in or out), so a plain `useState` draft suffices.
- `MIN_GAP_MS` also implicitly guards against a divide-by-zero-adjacent degenerate range if a caller ever renders with `dur` very close to 0 (the top-level `if (dur <= 0) return null` handles the exact-zero case).
