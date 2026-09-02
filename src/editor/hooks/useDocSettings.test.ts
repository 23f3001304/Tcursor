import { describe, it, expect } from "vitest";
import { createQueue } from "./opQueue";

const delay = (ms: number) => new Promise<void>((res) => setTimeout(res, ms));

interface Doc { zooms: string[]; settings: { volume: number } }

// Adversarial simulation for `useDocSettings.write`'s Critical 1 fix (review round 1): a settings
// write queued behind an in-flight `applyOp` used to close over `doc` at CALL time, which - since
// the queue can delay execution - was stale by the time it actually ran, so `{...staleDoc,
// settings: next}` silently dropped every non-settings field the apply had just landed (a
// blind-overwrite, on both disk and in `setDoc`). Mirrors the real shapes closely enough to pin
// the fix's ordering guarantee without needing React or a real EditDoc.
describe("write reading docRef.current AT EXECUTION TIME, not call time (Critical 1)", () => {
  it("a settings write queued behind an in-flight applyOp keeps that apply's result - repro: release a pill drag, toggle a setting inside the IPC window", async () => {
    const enqueue = createQueue();
    const docRef = { current: { zooms: [], settings: { volume: 50 } } as Doc };

    // applyOp: mirrors Editor.tsx's real shape - reads docRef.current fresh, awaits an IPC round
    // trip, then writes BOTH setDoc and docRef.current before returning (the M1 fix).
    const applyOp = () => enqueue(async () => {
      await delay(5); // the pill drag's apply_edit_op round trip
      const d: Doc = { ...docRef.current, zooms: ["z1"] }; // the drag's real result
      docRef.current = d;
      return d;
    });

    // write: mirrors useDocSettings.ts's FIXED shape - reads docRef.current INSIDE the enqueued
    // task, so it merges onto whatever the queue has ACTUALLY settled to by the time it runs.
    const write = (volume: number) => enqueue(async () => {
      const doc = docRef.current;
      const newDoc: Doc = { ...doc, settings: { ...doc.settings, volume } };
      docRef.current = newDoc;
      return newDoc;
    });

    // The exact repro: the settings write is issued WHILE the pill drag's apply is still in
    // flight (both fire "at once" - write is enqueued before applyOp's 5ms delay resolves).
    const [, settingsResult] = await Promise.all([applyOp(), write(75)]);

    expect(settingsResult.zooms).toEqual(["z1"]); // the drag's result must survive
    expect(settingsResult.settings.volume).toBe(75); // and the setting itself must land
    expect(docRef.current.zooms).toEqual(["z1"]);
    expect(docRef.current.settings.volume).toBe(75);
  });

  it("WITHOUT the fix (closing over doc at call time), the settings write blind-overwrites the apply's result", async () => {
    // Negative control - proves the test above is load-bearing. `doc` here is captured ONCE, up
    // front, exactly like the pre-fix `useDocSettings.write` closed over its `doc` parameter.
    const enqueue = createQueue();
    const docRef = { current: { zooms: [], settings: { volume: 50 } } as Doc };
    const staleDocAtCallTime = docRef.current;

    const applyOp = () => enqueue(async () => {
      await delay(5);
      const d: Doc = { ...docRef.current, zooms: ["z1"] };
      docRef.current = d;
      return d;
    });

    const brokenWrite = (volume: number) => enqueue(async () => {
      const newDoc: Doc = { ...staleDocAtCallTime, settings: { ...staleDocAtCallTime.settings, volume } };
      docRef.current = newDoc;
      return newDoc;
    });

    const [, settingsResult] = await Promise.all([applyOp(), brokenWrite(75)]);
    expect(settingsResult.zooms).toEqual([]); // the drag's result is gone - the bug
  });
});
