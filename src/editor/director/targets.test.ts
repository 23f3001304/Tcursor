import { describe, expect, it } from "vitest";
import { timelinePointForMs, rectCenter, pillPoint, anchorPoint } from "./targets";

/** Stubs `getBoundingClientRect` on a jsdom element - jsdom itself always reports an all-zero
 *  rect, so every test here that cares about geometry supplies its own. */
function stubRect(el: Element, r: { left: number; top: number; width: number; height: number }) {
  const full: DOMRect = {
    ...r, right: r.left + r.width, bottom: r.top + r.height, x: r.left, y: r.top,
    toJSON: () => ({}),
  };
  el.getBoundingClientRect = () => full;
}

describe("timelinePointForMs", () => {
  it("maps ms to an x fraction of the track width, falling back to the track's own vertical center", () => {
    const track = document.createElement("div");
    stubRect(track, { left: 100, top: 50, width: 1000, height: 40 });
    expect(timelinePointForMs(track, 5000, 10_000, "trim-in")).toEqual({ x: 600, y: 70 });
  });

  it("centers on the bottom-most (layer 0) zoom row when one exists", () => {
    const track = document.createElement("div");
    stubRect(track, { left: 0, top: 0, width: 1000, height: 200 });
    const row1 = document.createElement("div"); row1.className = "e-zoomrow";
    const row0 = document.createElement("div"); row0.className = "e-zoomrow";
    // Timeline.tsx renders the highest layer first and layer 0 LAST - row0 (layer 0) is where a
    // fresh add_zoom_full always lands, so it must be the one whose center wins.
    track.append(row1, row0);
    stubRect(row1, { left: 0, top: 10, width: 1000, height: 32 });
    stubRect(row0, { left: 0, top: 50, width: 1000, height: 32 });
    expect(timelinePointForMs(track, 0, 10_000, "zoom").y).toBe(66);
  });

  it("clamps ms outside [0, dur] to the track's own edges", () => {
    const track = document.createElement("div");
    stubRect(track, { left: 0, top: 0, width: 500, height: 40 });
    expect(timelinePointForMs(track, -100, 1000, "trim-in").x).toBe(0);
    expect(timelinePointForMs(track, 5000, 1000, "trim-in").x).toBe(500);
  });

  it("returns x at the track's left edge when dur is 0 (no division by zero)", () => {
    const track = document.createElement("div");
    stubRect(track, { left: 20, top: 0, width: 500, height: 40 });
    expect(timelinePointForMs(track, 100, 0, "trim-in").x).toBe(20);
  });
});

describe("rectCenter", () => {
  it("returns the element's rect midpoint", () => {
    const el = document.createElement("div");
    stubRect(el, { left: 10, top: 20, width: 30, height: 40 });
    expect(rectCenter(el)).toEqual({ x: 25, y: 40 });
  });
});

describe("pillPoint", () => {
  it("finds a pill by its data-region-id and returns its center", () => {
    const track = document.createElement("div");
    stubRect(track, { left: 0, top: 0, width: 1000, height: 100 });
    const pill = document.createElement("div");
    pill.setAttribute("data-region-id", "z-123");
    stubRect(pill, { left: 100, top: 10, width: 40, height: 20 });
    track.appendChild(pill);
    expect(pillPoint(track, "z-123")).toEqual({ x: 120, y: 20 });
  });

  it("returns null when no pill matches the id", () => {
    const track = document.createElement("div");
    expect(pillPoint(track, "nope")).toBeNull();
  });
});

describe("anchorPoint", () => {
  it("finds the mounted data-director-anchor and returns its center", () => {
    document.body.innerHTML = "";
    const btn = document.createElement("button");
    btn.setAttribute("data-director-anchor", "wand");
    stubRect(btn, { left: 10, top: 10, width: 20, height: 20 });
    document.body.appendChild(btn);
    expect(anchorPoint("wand")).toEqual({ x: 20, y: 20 });
    document.body.innerHTML = "";
  });

  it("returns null when no anchor is mounted", () => {
    document.body.innerHTML = "";
    expect(anchorPoint("missing")).toBeNull();
  });
});
