# src/shared/aiRun.ts

The wire shape of one AI Director v2 run - the TypeScript mirror of Rust `ai::plan::schema` (`AiRun`, `AiProposal`, `ProposalKind`, `NEW_ID`). Shared by the IPC call that produces a run (`ipc.ts::aiPropose`, M4 Task 3) and the review sheet that shows it (`editor/director/review/`, M4 Task 4). Kept in its own file, rather than inside `ipc.ts` as the plan first placed it, so the two halves of the M4 plan could be built on disjoint trees by two agents (2026-09-15).

## AiProposalKind

```ts
export type AiProposalKind = "zoom" | "layout" | "spotlight" | "trim" | "cut" | "speed";
```

The proposal kinds on the wire, snake_case like Rust's `ProposalKind`. `cut` and `speed` are included because time remap landed: the exporter, the audio and the preview all render `doc.cuts` / `doc.speed` now (the plan's A3 update).

## AiProposal

```ts
export interface AiProposal {
  id: string; kind: AiProposalKind; why: string; at_ms: number; dur_ms: number;
  rect: [number, number, number, number] | null; ops: EditOp[];
}
```

One proposed edit.

- `id` - `p0`, `p1`, ... in final time order; what the sheet's skip set and preview refer to.
- `kind` / `why` - the row's icon and its one-line reason (`why` is at most 120 chars, one line).
- `at_ms` / `dur_ms` - the instant the preview scrubs to, and the span, on the OUTPUT clock.
- `rect` - the normalised region the model named, for the stage outline only; it never reaches `EditDoc` (a zoom stores its centre as `ZoomTarget::Fixed`, a spotlight its radius).
- `ops` - the `EditOp`s that realise it, applied in order; a follow-up op may carry `AI_NEW_ID`.

## AiRun

```ts
export interface AiRun { model: string; vision: boolean; frames: number; elapsed_ms: number; proposals: AiProposal[] }
```

The whole answer, with what the sheet header says honestly: the model, whether it saw frames and how many, the elapsed time, and the proposals. An empty `proposals` is a real answer ("Nothing worth editing was found in this clip."), not an error.

## AI_NEW_ID

```ts
export const AI_NEW_ID = "$new";
```

The placeholder a follow-up op carries where the id of the region the previous op just created belongs (Rust `NEW_ID`). The apply step (M4 Task 5, `applyOps.ts`) substitutes the real id by set-difference over `doc.zooms` / `doc.effects` / `doc.layout` after each add, the same set-difference the one-shot reveal used to find the pill it made.
