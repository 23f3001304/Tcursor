// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import {
  CAM_ACTION_OPTIONS,
  durationOptions,
  isCamActionSelected,
  targetForMode,
  targetMode,
  zoomScopedSeekMs,
} from "./zoomInspectorModel";
import type { EditOp } from "../../shared/edit";

describe("zoom target modes (T31 - Region targets)", () => {
  it("reads cursor as Follow cursor and any fixed point as Region", () => {
    expect(targetMode("cursor")).toBe("cursor");
    expect(targetMode({ fixed: { x: 0.5, y: 0.5 } })).toBe("region");
    expect(targetMode({ fixed: { x: 0.1, y: 0.9 } })).toBe("region");
  });

  it("keeps an already-aimed point when re-selecting Region (no silent re-centre)", () => {
    const aimed = { fixed: { x: 0.2, y: 0.8 } } as const;
    expect(targetForMode("region", aimed)).toEqual(aimed);
  });

  it("starts a never-aimed zoom's Region at frame centre - what the old Center button wrote", () => {
    expect(targetForMode("region", "cursor")).toEqual({ fixed: { x: 0.5, y: 0.5 } });
  });

  it("drops back to the cursor target with no stored point", () => {
    expect(targetForMode("cursor", { fixed: { x: 0.2, y: 0.8 } })).toBe("cursor");
  });

  it("builds the exact update_zoom payload the stage aim commits", () => {
    const op: EditOp = { op: "update_zoom", id: "z1", target: { fixed: { x: 0.33, y: 0.66 } } };
    expect(op).toEqual({ op: "update_zoom", id: "z1", target: { fixed: { x: 0.33, y: 0.66 } } });
  });
});

describe("CAM_ACTION_OPTIONS (Task 26 commit 1 - per-zoom webcam action)", () => {
  it("exposes exactly the 4 options the spec calls for, in order", () => {
    expect(CAM_ACTION_OPTIONS.map((o) => o.label)).toEqual(["Global default", "Stay", "Shrink", "Hide"]);
    expect(CAM_ACTION_OPTIONS.map((o) => o.value)).toEqual([null, "stay", { shrink: { to: 0.62 } }, "hide"]);
  });

  it("builds the exact set_zoom_cam_action op payload for each option", () => {
    const ops: EditOp[] = CAM_ACTION_OPTIONS.map((opt) => ({
      op: "set_zoom_cam_action",
      id: "z1",
      action: opt.value,
    }));
    expect(ops).toEqual([
      { op: "set_zoom_cam_action", id: "z1", action: null },
      { op: "set_zoom_cam_action", id: "z1", action: "stay" },
      { op: "set_zoom_cam_action", id: "z1", action: { shrink: { to: 0.62 } } },
      { op: "set_zoom_cam_action", id: "z1", action: "hide" },
    ]);
  });
});

describe("isCamActionSelected", () => {
  it("treats null and undefined `current` as the Global default option", () => {
    expect(isCamActionSelected(null, null)).toBe(true);
    expect(isCamActionSelected(undefined, null)).toBe(true);
    expect(isCamActionSelected("stay", null)).toBe(false);
  });

  it("matches string variants (stay/hide) exactly", () => {
    expect(isCamActionSelected("stay", "stay")).toBe(true);
    expect(isCamActionSelected("hide", "stay")).toBe(false);
    expect(isCamActionSelected("hide", "hide")).toBe(true);
  });

  it("treats any shrink object as the Shrink option, regardless of `to`", () => {
    expect(isCamActionSelected({ shrink: { to: 0.62 } }, { shrink: { to: 0.62 } })).toBe(true);
    expect(isCamActionSelected({ shrink: { to: 0.4 } }, { shrink: { to: 0.62 } })).toBe(true);
    expect(isCamActionSelected("stay", { shrink: { to: 0.62 } })).toBe(false);
  });
});

describe("zoomScopedSeekMs (gate finding - zoom-scoped controls seek discoverability)", () => {
  it("does not seek when the playhead is already inside the span (inclusive both ends)", () => {
    expect(zoomScopedSeekMs(1000, 1000, 2000)).toBeNull();
    expect(zoomScopedSeekMs(2000, 1000, 2000)).toBeNull();
    expect(zoomScopedSeekMs(1500, 1000, 2000)).toBeNull();
  });

  it("seeks to the midpoint when the playhead is before the span", () => {
    expect(zoomScopedSeekMs(0, 1000, 2000)).toBe(1500);
  });

  it("seeks to the midpoint when the playhead is after the span", () => {
    expect(zoomScopedSeekMs(5000, 1000, 2000)).toBe(1500);
  });

  it("rounds a non-integer midpoint to the nearest ms", () => {
    expect(zoomScopedSeekMs(0, 1000, 2001)).toBe(1501);
  });
});

describe("durationOptions (smart typing duration)", () => {
  it("lights Fixed by default and Smart typing once the zoom is smart", () => {
    expect(durationOptions(false).map((o) => [o.key, o.on])).toEqual([
      ["fixed", true],
      ["smart", false],
    ]);
    expect(durationOptions(true).map((o) => [o.key, o.on])).toEqual([
      ["fixed", false],
      ["smart", true],
    ]);
  });
});
