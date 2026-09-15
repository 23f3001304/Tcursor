// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { matchesStackTop, shouldPushNewSnapshot } from "./useEditHistory";
import { createQueue } from "../../util/opQueue";

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
    const stack: string[] = [];
    stack.push("A");
    const tokenA = "A";
    stack.push("B");
    expect(matchesStackTop(stack, tokenA)).toBe(false);
    expect(matchesStackTop(stack, "B")).toBe(true);
  });
});

describe("swap ordering under the real queue (adversarial: two rapid undos, fast-resolving saveEdit)", () => {
  it("produces [D2, D1] on the redo stack, not [D2, D2] - the 2nd queued undo must see the 1st's result via docRef", async () => {
    const enqueue = createQueue();
    const docRef = { current: "D2" };
    const undoStack = ["D0", "D1"];
    const redoStack: string[] = [];
    const swap = async (from: string[], to: string[]) => {
      const next = from.pop();
      const current = docRef.current;
      if (!next) return;
      to.push(current);
      docRef.current = next;
      await delay(1);
    };
    await Promise.all([enqueue(() => swap(undoStack, redoStack)), enqueue(() => swap(undoStack, redoStack))]);
    expect(redoStack).toEqual(["D2", "D1"]);
    expect(docRef.current).toBe("D0");
  });

  it("WITHOUT the docRef-at-execution-time write, the same two undos would corrupt the redo stack to [D2, D2] and lose D1", async () => {
    const enqueue = createQueue();
    const docRef = { current: "D2" };
    const undoStack = ["D0", "D1"];
    const redoStack: string[] = [];
    const brokenSwap = async (from: string[], to: string[]) => {
      const next = from.pop();
      const current = docRef.current;
      if (!next) return;
      to.push(current);
      await delay(1);
    };
    await Promise.all([
      enqueue(() => brokenSwap(undoStack, redoStack)),
      enqueue(() => brokenSwap(undoStack, redoStack)),
    ]);
    expect(redoStack).toEqual(["D2", "D2"]);
  });
});
