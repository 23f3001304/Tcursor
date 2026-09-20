import { describe, it, expect } from "vitest";
import { pickAddedCameraMoveId, runSplitAt } from "./useTimelineActions";
import type { CameraMove, Clip, EditDoc, EditOp } from "../../../shared/edit";

const kf = (id: string, t_ms: number): CameraMove => ({
  id,
  t_ms,
  x: 0.5,
  y: 0.5,
  size: 0.25,
  easing: "smooth",
  shape: "layout",
  roundness: 0.12,
});

describe("pickAddedCameraMoveId", () => {
  it("finds the one id present in `after` but not `before`", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    const after = [kf("k0", 5000), kf("k1", 8000), kf("k2", 2000)];
    expect(pickAddedCameraMoveId(before, after)).toBe("k2");
  });

  it("finds it even after the server re-sorts by t_ms, landing it in the MIDDLE, not last", () => {
    const before = [kf("k0", 5000), kf("k1", 8000)];
    const after = [kf("k2", 2000), kf("k0", 5000), kf("k1", 8000)];
    expect(pickAddedCameraMoveId(before, after)).toBe("k2");
  });

  it("returns null when nothing changed (e.g. a failed apply left `after` === `before`)", () => {
    const before = [kf("k0", 5000)];
    expect(pickAddedCameraMoveId(before, before)).toBeNull();
  });

  it("returns null when both are empty", () => {
    expect(pickAddedCameraMoveId([], [])).toBeNull();
  });

  it("handles the first keyframe ever added (empty before)", () => {
    expect(pickAddedCameraMoveId([], [kf("k0", 1000)])).toBe("k0");
  });
});

const clip = (id: string, src_in_ms: number, src_out_ms: number): Clip => ({
  id,
  src_in_ms,
  src_out_ms,
  transition_in_ms: 0,
});

describe("runSplitAt", () => {
  it("selects the clip the split created, not the doc's last clip (split inside the first of three)", async () => {
    const before = [clip("c0", 0, 1000), clip("c1", 1000, 2000), clip("c2", 2000, 3000)];
    const after = [
      clip("c0", 0, 500),
      clip("new", 500, 1000),
      clip("c1", 1000, 2000),
      clip("c2", 2000, 3000),
    ];
    const ops: EditOp[] = [];
    const sel: (string | null)[] = [];
    const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
      ops.push(op);
      return { clips: after } as unknown as EditDoc;
    };
    await runSplitAt(applyOp, 500.4, before, (id) => sel.push(id));
    expect(ops).toEqual([{ op: "split_at", at_ms: 500 }]);
    expect(sel).toEqual(["new"]);
  });

  it("selects the right-hand half on the first split of a clip-less document", async () => {
    const after = [clip("a", 0, 3000), clip("b", 3000, 5000)];
    const ops: EditOp[] = [];
    const sel: (string | null)[] = [];
    const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
      ops.push(op);
      return { clips: after } as unknown as EditDoc;
    };
    await runSplitAt(applyOp, 3000, [], (id) => sel.push(id));
    expect(ops).toEqual([{ op: "split_at", at_ms: 3000 }]);
    expect(sel).toEqual(["b"]);
  });

  it("leaves the selection alone on a no-op split (a clip edge or outside every clip)", async () => {
    const clips = [clip("c0", 0, 1000), clip("c1", 1000, 2000)];
    const ops: EditOp[] = [];
    const sel: (string | null)[] = [];
    const applyOp = async (op: EditOp): Promise<EditDoc | null> => {
      ops.push(op);
      return { clips } as unknown as EditDoc;
    };
    await runSplitAt(applyOp, 1000, clips, (id) => sel.push(id));
    expect(ops).toEqual([{ op: "split_at", at_ms: 1000 }]);
    expect(sel).toEqual([]);
  });
});
