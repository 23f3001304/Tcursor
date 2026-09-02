import { describe, it, expect } from "vitest";
import { matchesStackTop, shouldPushNewSnapshot } from "./useEditHistory";
import { createQueue } from "./opQueue";

const delay = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

describe("shouldPushNewSnapshot", () => {
  it("is true for an empty stack regardless of timing", () => {
    expect(shouldPushNewSnapshot(0, 0, 0, 400)).toBe(true);
  });

  it("is true once the gap since the last push exceeds coalesceMs", () => {
    expect(shouldPushNewSnapshot(1, 1000, 1401, 400)).toBe(true);
  });

  it("is false (coalesce) right at and under the window", () => {
    expect(shouldPushNewSnapshot(1, 1000, 1400, 400)).toBe(false);
    expect(shouldPushNewSnapshot(1, 1000, 1100, 400)).toBe(false);
  });
});

describe("matchesStackTop - the identity guard unrecord relies on", () => {
  it("is false for a null snapshot (record coalesced - nothing to match)", () => {
    expect(matchesStackTop(["A"], null)).toBe(false);
  });

  it("is false for an empty stack", () => {
    expect(matchesStackTop([], "A")).toBe(false);
  });

  it("is true when the snapshot is exactly the top", () => {
    expect(matchesStackTop(["X", "A"], "A")).toBe(true);
  });

  it("record-between-record-and-unrecord: a LATER push landing on top makes an earlier op's own token a safe no-op", () => {
    // review round 1, Important 4: a record from a DIFFERENT op landing on top of an earlier
    // op's own pushed snapshot must make that earlier op's own (now stale) unrecord token a
    // guaranteed no-op - it must never pop the LATER, real edit's entry instead.
    const stack: string[] = [];
    stack.push("A"); // op #1's record() push
    const tokenA = "A";
    stack.push("B"); // op #2's record() push, landing on top before #1's unrecord ever runs
    expect(matchesStackTop(stack, tokenA)).toBe(false); // A's own stale token must not match
    expect(matchesStackTop(stack, "B")).toBe(true); // B's own token still correctly matches
  });
});

// Adversarial matrix for the real queue + docRef pattern `Editor.tsx`'s `applyOp`/`useEditHistory`'s
// `swap` share (review round 1, Important 2) - a plain string stands in for `EditDoc` so the
// simulation stays focused on the ORDERING, not on doc shape.
describe("swap ordering under the real queue (adversarial: two rapid undos, fast-resolving saveEdit)", () => {
  it("produces [D2, D1] on the redo stack, not [D2, D2] - the 2nd queued undo must see the 1st's result via docRef", async () => {
    const enqueue = createQueue();
    const docRef = { current: "D2" };
    const undoStack = ["D0", "D1"];
    const redoStack: string[] = [];
    // Mirrors `useEditHistory.swap`'s real shape: push the CURRENT doc onto the counterpart stack,
    // `setDoc` the popped one, THEN write `docRef.current` before any await (the M1 fix) - not
    // after - so a queued task immediately behind this one sees the real result, not a stale ref.
    const swap = async (from: string[], to: string[]) => {
      const next = from.pop();
      const current = docRef.current;
      if (!next) return;
      to.push(current);
      docRef.current = next;
      await delay(1); // simulate a fast saveEdit
    };
    await Promise.all([
      enqueue(() => swap(undoStack, redoStack)),
      enqueue(() => swap(undoStack, redoStack)),
    ]);
    expect(redoStack).toEqual(["D2", "D1"]);
    expect(docRef.current).toBe("D0");
  });

  it("WITHOUT the docRef-at-execution-time write, the same two undos would corrupt the redo stack to [D2, D2] and lose D1", async () => {
    // Negative control: proves the fix in the test above is load-bearing, not incidental - the
    // exact same queue, the exact same two calls, but `docRef.current` is only ever updated by a
    // (here, absent) "React re-render", reproducing the pre-fix bug byte for byte.
    const enqueue = createQueue();
    const docRef = { current: "D2" };
    const undoStack = ["D0", "D1"];
    const redoStack: string[] = [];
    const brokenSwap = async (from: string[], to: string[]) => {
      const next = from.pop();
      const current = docRef.current; // never reassigned - the bug
      if (!next) return;
      to.push(current);
      await delay(1);
    };
    await Promise.all([
      enqueue(() => brokenSwap(undoStack, redoStack)),
      enqueue(() => brokenSwap(undoStack, redoStack)),
    ]);
    expect(redoStack).toEqual(["D2", "D2"]); // D1 is gone from both stacks
  });
});
