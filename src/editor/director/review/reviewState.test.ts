import { describe, it, expect } from "vitest";
import { toggle, acceptedIds, orderedOps, summary, previewTarget } from "./reviewState";
import type { AiRun } from "../../../shared/aiRun";

const run: AiRun = {
  model: "qwen2.5vl:7b",
  vision: true,
  frames: 9,
  elapsed_ms: 12_400,
  proposals: [
    {
      id: "p0",
      kind: "trim",
      why: "dead air before you start",
      at_ms: 0,
      dur_ms: 0,
      rect: null,
      ops: [{ op: "set_trim", in_ms: 1200, out_ms: 0 }],
    },
    {
      id: "p1",
      kind: "zoom",
      why: "you click Save",
      at_ms: 2700,
      dur_ms: 1500,
      rect: [0.3, 0.2, 0.2, 0.16],
      ops: [
        { op: "add_zoom_full", at_ms: 2700, dur_ms: 1500, scale: 2.4 },
        { op: "update_zoom", id: "$new", target: { fixed: { x: 0.4, y: 0.28 } } },
      ],
    },
  ],
};

describe("review state", () => {
  it("accepts everything by default", () => {
    expect(acceptedIds(run, new Set())).toEqual(["p0", "p1"]);
  });

  it("skipping an item takes its ops out of the apply list", () => {
    const skipped = toggle(new Set<string>(), "p1");
    expect(acceptedIds(run, skipped)).toEqual(["p0"]);
    expect(orderedOps(run, skipped)).toEqual([{ op: "set_trim", in_ms: 1200, out_ms: 0 }]);
  });

  it("toggling twice puts it back", () => {
    expect(toggle(toggle(new Set<string>(), "p1"), "p1").has("p1")).toBe(false);
  });

  it("flattens each accepted proposal's ops in order", () => {
    expect(orderedOps(run, new Set()).map((o) => o.op)).toEqual(["set_trim", "add_zoom_full", "update_zoom"]);
  });

  it("counts honestly", () => {
    expect(summary(run, new Set())).toBe("2 edits, 2 accepted");
    expect(summary(run, toggle(new Set<string>(), "p1"))).toBe("2 edits, 1 accepted");
  });

  it("says nothing was found rather than showing an empty list", () => {
    expect(summary({ ...run, proposals: [] }, new Set())).toBe(
      "Nothing worth editing was found in this clip.",
    );
  });

  it("previewing gives the playhead a time and the stage a rect, or no rect at all", () => {
    expect(previewTarget(run, "p1")).toEqual({ tMs: 2700, rect: [0.3, 0.2, 0.2, 0.16] });
    expect(previewTarget(run, "p0")).toEqual({ tMs: 0, rect: null });
    expect(previewTarget(run, "nope")).toBeNull();
  });
});
