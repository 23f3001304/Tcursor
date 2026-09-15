// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { createQueue } from "../../util/opQueue";

const delay = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

interface Doc {
  zooms: string[];
  settings: { volume: number };
}

describe("write reading docRef.current AT EXECUTION TIME, not call time (Critical 1)", () => {
  it("a settings write queued behind an in-flight applyOp keeps that apply's result - repro: release a pill drag, toggle a setting inside the IPC window", async () => {
    const enqueue = createQueue();
    const docRef = { current: { zooms: [], settings: { volume: 50 } } as Doc };

    const applyOp = () =>
      enqueue(async () => {
        await delay(5);
        const d: Doc = { ...docRef.current, zooms: ["z1"] };
        docRef.current = d;
        return d;
      });

    const write = (volume: number) =>
      enqueue(async () => {
        const doc = docRef.current;
        const newDoc: Doc = { ...doc, settings: { ...doc.settings, volume } };
        docRef.current = newDoc;
        return newDoc;
      });

    const [, settingsResult] = await Promise.all([applyOp(), write(75)]);

    expect(settingsResult.zooms).toEqual(["z1"]);
    expect(settingsResult.settings.volume).toBe(75);
    expect(docRef.current.zooms).toEqual(["z1"]);
    expect(docRef.current.settings.volume).toBe(75);
  });

  it("WITHOUT the fix (closing over doc at call time), the settings write blind-overwrites the apply's result", async () => {
    const enqueue = createQueue();
    const docRef = { current: { zooms: [], settings: { volume: 50 } } as Doc };
    const staleDocAtCallTime = docRef.current;

    const applyOp = () =>
      enqueue(async () => {
        await delay(5);
        const d: Doc = { ...docRef.current, zooms: ["z1"] };
        docRef.current = d;
        return d;
      });

    const brokenWrite = (volume: number) =>
      enqueue(async () => {
        const newDoc: Doc = { ...staleDocAtCallTime, settings: { ...staleDocAtCallTime.settings, volume } };
        docRef.current = newDoc;
        return newDoc;
      });

    const [, settingsResult] = await Promise.all([applyOp(), brokenWrite(75)]);
    expect(settingsResult.zooms).toEqual([]);
  });
});
