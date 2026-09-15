import { describe, expect, it } from "vitest";
import { createQueue } from "./opQueue";

const delay = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

describe("createQueue", () => {
  it("runs enqueued fns strictly in call order, even when a LATER one would resolve first", async () => {
    const enqueue = createQueue();
    const order: number[] = [];
    const p1 = enqueue(async () => {
      await delay(30);
      order.push(1);
    });
    const p2 = enqueue(async () => {
      await delay(10);
      order.push(2);
    });
    const p3 = enqueue(async () => {
      order.push(3);
    });
    await Promise.all([p1, p2, p3]);
    expect(order).toEqual([1, 2, 3]);
  });

  it("does not start a queued fn until the previous one has settled", async () => {
    const enqueue = createQueue();
    let inFlight = 0;
    let maxInFlight = 0;
    const run = () =>
      enqueue(async () => {
        inFlight++;
        maxInFlight = Math.max(maxInFlight, inFlight);
        await delay(5);
        inFlight--;
      });
    await Promise.all([run(), run(), run()]);
    expect(maxInFlight).toBe(1);
  });

  it("keeps the chain alive when an earlier fn rejects - later fns still run in order", async () => {
    const enqueue = createQueue();
    const order: number[] = [];
    const p1 = enqueue(async () => {
      order.push(1);
      throw new Error("boom");
    });
    const p2 = enqueue(async () => {
      order.push(2);
    });
    await expect(p1).rejects.toThrow("boom");
    await p2;
    expect(order).toEqual([1, 2]);
  });

  it("resolves each call with that call's own return value", async () => {
    const enqueue = createQueue();
    await expect(enqueue(async () => 42)).resolves.toBe(42);
    await expect(enqueue(async () => "done")).resolves.toBe("done");
  });

  it("a fn enqueued while an earlier one is in flight sees that earlier fn's write, if it reads a ref at execution time", async () => {
    const enqueue = createQueue();
    const docRef = { current: "doc-v0" };
    const seenByOp2: string[] = [];

    const p1 = enqueue(async () => {
      await delay(20);
      docRef.current = "doc-v1";
    });
    const p2 = enqueue(async () => {
      seenByOp2.push(docRef.current);
    });

    await Promise.all([p1, p2]);
    expect(seenByOp2).toEqual(["doc-v1"]);
  });
});
