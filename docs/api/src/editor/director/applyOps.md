# src/editor/director/applyOps.ts

The apply half of the AI review sheet, as plain functions: which id a follow-up op should carry, and the loop that sends every accepted op as ONE undo step. No React, no IPC of its own; `useAiRun` hands in the doc ref and the raw edit call, so all of it is unit-tested (`applyOps.test.ts`) with a fake `applyOp` and nothing mounted.

## substituteNewId

```ts
export function substituteNewId(op: EditOp, newId: string): EditOp
```

`op` with `AI_NEW_ID` (`shared/aiRun.ts`, the `"$new"` placeholder Rust `NEW_ID` emits) replaced by `newId`. Any other id is returned exactly as it is: a real id the model named on purpose must never be rewritten to the region it did not mean. An op with no `id` field passes through untouched.

## newRegionId

```ts
export function newRegionId(before: EditDoc, after: EditDoc): string | null
```

The id of the one region `after` has that `before` did not, checked across the three lists an apply can add to, in order: `zooms`, `effects`, `layout`. `null` when the apply created nothing. That is what a follow-up op's `AI_NEW_ID` stands for: the region the op just before it made.

## ApplyIo

```ts
export interface ApplyIo {
  record: (d: EditDoc) => void;
  applyOp: (op: EditOp) => Promise<EditDoc>;
  setDoc: (d: EditDoc) => void;
  docRef: { current: EditDoc | null };
}
```

What `applyRun` needs from the editor: the undo `record` (called once, and only when something lands), the raw edit call (one op in, the doc it produced out; `applyEditOp` in practice), the doc setter, and the live doc ref.

## applyRun

```ts
export async function applyRun(run: AiRun, skipped: ReadonlySet<string>, io: ApplyIo): Promise<number>
```

Applies every accepted proposal's ops, in `orderedOps` order (`review/reviewState.ts`: proposal order, then op order within each), as ONE undo step. Resolves to how many ops actually landed.

### Behavior

- **Nothing accepted, nothing recorded.** With no ops to send (or no doc yet) it returns `0` before touching `record`, so a skip-everything Apply leaves no no-op snapshot for Undo to light up over. Otherwise `io.record(doc0)` fires exactly once, up front.
- **The sentinel is resolved from the previous apply.** After each op lands, `newRegionId(prev, next)` is remembered. A following op that carries `AI_NEW_ID` is sent with that id; when the op before it created nothing, the follow-up is dropped rather than aimed at whatever region happened to be there (`applyOps.test.ts`, "drops a follow-up op whose region cannot be identified").
- **`docRef.current` is written after every op**, alongside `setDoc`, so a queued undo that runs before React re-renders still reads the latest doc (the same rule `Editor.tsx`'s `applyOp` follows).
- **`rev` is the caller's.** The hook bumps it once at the end, not per op, so the camera curve refetches once.
- A rejected `applyOp` propagates; the ops that landed before it stay, under the one recorded step.

### Used by

- `src/editor/director/useAiRun.ts` - `apply()`, inside `Editor`'s op queue.
