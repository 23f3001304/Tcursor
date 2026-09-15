# src/editor/director/choreography.ts

The pure step planner behind the AI director's pointer replay - maps one applied edit op to what the fake pointer (`DirectorPointer.tsx`) should do to perform it after the fact, and paces how long each beat takes. No DOM, no timers, no randomness - fully unit-tested (`choreography.test.ts`). The edit has already landed by the time a plan is walked (`useAiRun.ts`'s replay runs after `applyRun`), so a plan is a gesture at the real pill, never an apply.

## StepKind

```ts
export type StepKind = "aim-timeline" | "drag-trim" | "none"
```

The three shapes a replay ever walks: aim at one point on a timeline lane and press, aim at a trim handle, or hold position.

## StepPlan

```ts
export interface StepPlan { kind: StepKind; lane: Lane; ms?: number; pressAfterMove: boolean }
```

The plan for one op. `ms` is the aim target on `lane` (`targets.ts`), present for every kind but `"none"`. A trim aims at the in handle only: after the apply, the handle already sits at `in_ms`, so pressing there is the honest gesture; the out handle has no matching press.

## planStep

```ts
export function planStep(op: EditOp): StepPlan
```

Maps one op to its `StepPlan`:

- `add_zoom_full { at_ms }` -> `{ kind: "aim-timeline", lane: "zoom", ms: at_ms, pressAfterMove: true }`.
- `add_effect { start_ms }` -> `{ kind: "aim-timeline", lane: "fx", ms: start_ms, pressAfterMove: true }` (a spotlight proposal).
- `set_trim { in_ms }` -> `{ kind: "drag-trim", lane: "trim-in", ms: in_ms, pressAfterMove: true }`.
- anything else -> `{ kind: "none", lane: "zoom", pressAfterMove: false }`: a layout segment, a cut or a speed span has no pill on the timeline the pointer can land on, so the replay skips it and moves on.

`useAiRun` plans each proposal from its FIRST op (`proposal.ops[0]`, the op that creates the region); a proposal's follow-up op (`update_zoom` with the created id) is the same pill and gets no gesture of its own.

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
