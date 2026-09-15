# src/editor/director/review/reviewState.ts

The review sheet's model, as pure functions over an `AiRun` (`src/shared/aiRun.ts`) and a set of SKIPPED proposal ids. No React, no IPC: the sheet renders them, `useAiRun` holds the set, and the apply step (M4 T5) consumes `orderedOps`.

**Skipped, not accepted.** Every proposal arrives accepted, so the empty set is "apply the whole run" and a freshly fetched run needs no per-item state to render correctly. The inverse (a set of accepted ids) would have to be seeded from the run itself, and a seeding bug would silently apply nothing.

## toggle

```ts
export function toggle(skipped: ReadonlySet<string>, id: string): ReadonlySet<string>
```

The skip set with `id` flipped. Always returns a NEW set - never mutates the one handed in - so React sees a changed reference and the row re-renders.

## acceptedIds

```ts
export function acceptedIds(run: AiRun, skipped: ReadonlySet<string>): string[]
```

The ids that will actually be applied, in the run's own order. Also what the footer's "Apply N edits" counts.

## orderedOps

```ts
export function orderedOps(run: AiRun, skipped: ReadonlySet<string>): EditOp[]
```

Every accepted proposal's `ops`, flattened in proposal order and then in op order within each proposal.

**That order is load-bearing.** A proposal whose first op creates a region (`add_zoom_full`, `add_effect`, `add_layout_seg`) carries a follow-up op holding `AI_NEW_ID` where that region's real id belongs, which the apply step substitutes from the id the previous op just created. Reordering the list, or applying a proposal's ops apart from each other, aims the follow-up at the wrong region.

## summary

```ts
export function summary(run: AiRun, skipped: ReadonlySet<string>): string
```

The one line under the sheet's header: `"2 edits, 1 accepted"`, pluralised.

An EMPTY run returns the sentence `"Nothing worth editing was found in this clip."` instead of a count. A model that looked and found nothing is giving a real answer, and a list with a zero count next to it reads as a failure of the feature rather than a verdict about the clip (binding decision 4).

## previewTarget

```ts
export function previewTarget(run: AiRun, id: string):
  { tMs: number; rect: [number, number, number, number] | null } | null
```

Where Preview sends the playhead, and the region the stage outlines while it is there.

- `rect: null` is normal, not an error: a trim has no region, and the model does not always name one. The playhead still moves; the stage simply draws nothing.
- `null` overall means the id is not in this run - a press against a run that has already been discarded or replaced.

### Used by

- `src/editor/director/review/ReviewSheet.tsx` - `acceptedIds`, `summary`.
- `src/editor/director/useAiRun.ts` - `toggle`, `previewTarget`.
- The apply step (M4 T5) - `orderedOps`.
