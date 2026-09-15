# src/editor/timeline/lanes/TrimOverlay.tsx

Renders on top of `Timeline.tsx`'s track stack: dims the trimmed-out head/tail and provides two draggable edge handles (in/out) that commit `SetTrim`. Drag mechanics mirror `useRegionDrag` (a local draft moves live; the IPC write only happens once, on pointerup) but are inlined here rather than reusing that hook, since there is no "region" (start+end pair with a layer) - just two independent 1D positions. Reads the same `resolveTrim` (`src/shared/edit.ts`) the export gate and the playhead clamp use, so the dimmed region always matches what will actually be cut.

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

**Drag.** `drag` state holds `{ mode: "in" | "out", ms }` while a handle is held; a `pointermove` listener (attached only while `drag` is non-null, mirroring `useRegionDrag`'s lifecycle) recomputes `ms` from the pointer's fraction of `trackRef`'s width, clamped so the two handles never cross within `MIN_GAP_MS` (200ms) of each other. `pointerdown` on a handle calls `e.stopPropagation()` first (so `Timeline`'s own track-body seek handler doesn't also fire), seeds `drag` from the handle's current live position, and records the pointer's starting `clientX` (`startXRef`) plus resets `movedRef` to `false`.

**Threshold before any live movement (bug-sweep-2 Task 8, M8).** `move` ignores every pointermove until `pastDragThreshold(e.clientX - startXRef.current, 0)` (`../util/dragThreshold.ts`, default 3px) is true, at which point `movedRef.current` latches `true` for the rest of the drag - so a handle that never moved 3px never visibly updates `drag.ms` at all (no snap-back-on-release jitter for a sub-threshold jiggle).

**Commit measured at RELEASE, not the latch (review round 1 minor - unifies with `useRegionDrag`/`CameraLane`).** On `pointerup`, applies `{ op: "set_trim", in_ms, out_ms }` ONLY when `pastDragThreshold(e.clientX - startXRef.current, 0)` is true AT THAT MOMENT - i.e. measured from the drag's ORIGIN to the RELEASE position, freshly, NOT `movedRef.current` (which only gates the LIVE visual update in `move` above). The original version used the `movedRef` latch for the commit decision too, which meant a drag that went out past 3px and then back near its origin before release still committed - `movedRef` never un-latches mid-drag. Measuring at release means that same "out and back" drag now correctly commits nothing, matching a real user's "I changed my mind" gesture. A bare click on a handle (no real movement at all) still never commits `set_trim` for an unchanged range either way (an undo step + IPC round trip for nothing; UX audit #5's mechanism). The OTHER field is left at its current `trim.*` value; if the dragged-out `outMs` reaches `dur` exactly, `out_ms` is written as `0` (back to "whole clip" / unset) rather than the literal duration, so a doc that's never actually been trimmed at that edge stays serialized as the `Trim::default()`-shaped "unset" value.

**Rendering.** Two `.e-trimdim` divs (absolute, `top:0;bottom:0`, spanning `.e-tlbody`'s full height like the playhead) cover `[0, liveIn]` and `[liveOut, dur]` - only rendered when non-empty (`liveIn > 0` / `liveOut < dur`). Their fill (Task D2, timeline.css) is a diagonal `repeating-linear-gradient` - an 8px dark band alternating with an 8px band tinted 4% white - a PATTERN fill, not chrome decoration, so it's exempt from the app's no-gradients-on-chrome rule; it reads as a textured "excluded" region instead of the old flat black scrim. Two `.e-trimhandle` divs (`cursor: ew-resize`) sit at `liveIn`/`liveOut`, each carrying an `.active` modifier when it's the handle currently being dragged (`drag?.mode === "in" | "out"` respectively) - CSS turns its raised grip's hairline border accent-colored only in that state (border doctrine: neutral at rest/hover, accent only while genuinely active). The grip itself (Task D2) is a small `--e-raised` capsule with 3 dots at its center, built entirely from `::before`/`::after` pseudo-elements - no new DOM, so the drag/threshold mechanics above are untouched.

### Notes

- No local commit debouncing is needed beyond "only on pointerup" - unlike `useRegionDrag`'s callers, there is only ever one drag target class (in or out), so a plain `useState` draft suffices.
- `MIN_GAP_MS` also implicitly guards against a divide-by-zero-adjacent degenerate range if a caller ever renders with `dur` very close to 0 (the top-level `if (dur <= 0) return null` handles the exact-zero case).
