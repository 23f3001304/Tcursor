# src/editor/director/choreography.ts

The pure step planner behind the AI director's choreographed reveal - maps one edit op to what the fake pointer (`DirectorPointer.tsx`) should do around it, and paces how long each beat takes. No DOM, no timers, no randomness - fully unit-tested (`choreography.test.ts`).

## StepKind

```ts
export type StepKind = "aim-timeline" | "sweep-lane" | "drag-trim" | "none"
```

The shape of choreography a step needs: aim at one point on the timeline and (usually) press, sweep across a lane, drag a trim handle, or hold position.

## StepPlan

```ts
export interface StepPlan { kind: StepKind; lane: Lane; ms?: number; pressAfterMove: boolean }
```

The plan for one op. `ms` (when present) is the PRIMARY aim target - absent for `"sweep-lane"`, which has no single point. `set_trim` can move both trim handles, but `planStep` only sees the op, not the doc's PRIOR trim, so it only plans the "in" leg here (`lane: "trim-in"`); `useDirector.ts`'s `reveal` (which HAS the current doc) decides the "out" leg itself, using the same `timelinePointForMs`, only when `out_ms` actually changed from the doc's current trim.

## planStep

```ts
export function planStep(op: EditOp): StepPlan
```

Maps one AI-director op to its `StepPlan`:

- `add_zoom_full { at_ms }` -> `{ kind: "aim-timeline", lane: "zoom", ms: at_ms, pressAfterMove: true }`.
- `clear_zooms` -> `{ kind: "sweep-lane", lane: "zoom", pressAfterMove: false }`.
- `set_trim { in_ms }` -> `{ kind: "drag-trim", lane: "trim-in", ms: in_ms, pressAfterMove: true }`.
- anything else -> `{ kind: "none", lane: "zoom", pressAfterMove: false }` (pointer holds position, op applies normally). The only op kinds the planner (`ai_plan`/`ops_from_json` on the Rust side) actually ever emits are the three above; this branch is defensive, not currently reachable from a real plan.

## pace

```ts
export function pace(distancePx: number): { travelCapMs: number }
```

Travel-time cap (ms) for a `distancePx` pointer move. The spring (`DirectorPointer`'s `useSpring`) drives the actual motion; this only bounds how long `moveTo` waits for the "settled within 2px" check before resolving anyway, so a long-distance move can never stall the reveal. Min 240ms, +0.6ms/px, capped at 700ms.

## dwellSettle

```ts
export function dwellSettle(stepCount: number): { dwellMs: number; settleMs: number }
```

Dwell-before-press and settle-after-apply durations for one step in a plan of `stepCount` steps. Base values are 160ms dwell / 240ms settle; once a plan runs longer than 8 steps, both scale down by `8 / stepCount` so a big reveal still finishes in roughly ~10s instead of stacking up linearly - each floors at 60ms so even a very long plan still visibly pauses between steps rather than flickering.
