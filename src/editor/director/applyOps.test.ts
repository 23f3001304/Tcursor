import { describe, it, expect, vi } from "vitest";
import { substituteNewId, newRegionId, applyRun } from "./applyOps";
import type { EditDoc } from "../../shared/edit";
import type { AiRun } from "../../shared/aiRun";

const DOC = { zooms: [], effects: [], layout: [] } as unknown as EditDoc;
const withZooms = (ids: string[]) => ({ ...DOC, zooms: ids.map((id) => ({ id })) }) as unknown as EditDoc;

describe("id substitution", () => {
  it("finds the region the previous apply created", () => {
    expect(newRegionId(withZooms(["z0"]), withZooms(["z0", "z1"]))).toBe("z1");
  });
  it("finds a new effect and a new layout segment too", () => {
    const before = { ...DOC, effects: [{ id: "e0" }] } as unknown as EditDoc;
    const after = { ...DOC, effects: [{ id: "e0" }, { id: "e1" }] } as unknown as EditDoc;
    expect(newRegionId(before, after)).toBe("e1");
    const lb = { ...DOC, layout: [{ id: "l0" }] } as unknown as EditDoc;
    const la = { ...DOC, layout: [{ id: "l0" }, { id: "l3" }] } as unknown as EditDoc;
    expect(newRegionId(lb, la)).toBe("l3");
  });
  it("returns null when the apply created nothing to follow up on", () => {
    expect(newRegionId(withZooms(["z0"]), withZooms(["z0"]))).toBeNull();
  });
  it("substitutes only the sentinel, never a real id", () => {
    expect(substituteNewId({ op: "update_zoom", id: "$new", scale: 2 }, "z1")).toEqual({
      op: "update_zoom",
      id: "z1",
      scale: 2,
    });
    expect((substituteNewId({ op: "update_zoom", id: "z7", scale: 2 }, "z1") as { id: string }).id).toBe(
      "z7",
    );
  });
});

const run: AiRun = {
  model: "m",
  vision: true,
  frames: 4,
  elapsed_ms: 900,
  proposals: [
    {
      id: "p0",
      kind: "zoom",
      why: "a",
      at_ms: 1000,
      dur_ms: 1000,
      rect: null,
      ops: [
        { op: "add_zoom_full", at_ms: 1000, dur_ms: 1000, scale: 2 },
        { op: "update_zoom", id: "$new", target: { fixed: { x: 0.5, y: 0.5 } } },
      ],
    },
    {
      id: "p1",
      kind: "trim",
      why: "b",
      at_ms: 0,
      dur_ms: 0,
      rect: null,
      ops: [{ op: "set_trim", in_ms: 500, out_ms: 0 }],
    },
  ],
};

describe("applying a run", () => {
  const io = () => {
    const sent: unknown[] = [];
    let doc = withZooms([]);
    return {
      sent,
      record: vi.fn(),
      setDoc: (d: EditDoc) => {
        doc = d;
      },
      docRef: {
        get current() {
          return doc;
        },
        set current(d: EditDoc) {
          doc = d;
        },
      },
      applyOp: vi.fn(async (op: { op: string }) => {
        sent.push(op);
        if (op.op === "add_zoom_full") doc = withZooms(["z0"]);
        return doc;
      }),
    };
  };

  it("records exactly one undo snapshot for the whole apply", async () => {
    const x = io();
    await applyRun(run, new Set(), x);
    expect(x.record).toHaveBeenCalledTimes(1);
  });

  it("substitutes the created zoom's id into the follow-up op it actually sends", async () => {
    const x = io();
    await applyRun(run, new Set(), x);
    expect(x.sent.map((o) => (o as { op: string }).op)).toEqual(["add_zoom_full", "update_zoom", "set_trim"]);
    expect((x.sent[1] as { id: string }).id).toBe("z0");
  });

  it("records nothing and sends nothing when every item was skipped", async () => {
    const x = io();
    const skipped = new Set(["p0", "p1"]);
    expect(await applyRun(run, skipped, x)).toBe(0);
    expect(x.record).not.toHaveBeenCalled();
    expect(x.applyOp).not.toHaveBeenCalled();
  });

  it("drops a follow-up op whose region cannot be identified rather than aiming at the wrong one", async () => {
    const x = io();
    x.applyOp = vi.fn(async (op: { op: string }) => {
      x.sent.push(op);
      return withZooms([]);
    });
    await applyRun(run, new Set(["p1"]), x);
    expect(x.sent.map((o) => (o as { op: string }).op)).toEqual(["add_zoom_full"]);
  });
});
