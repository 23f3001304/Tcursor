import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act, createElement, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { rangeOf, useRangeSelect, type Range } from "./useRangeSelect";

// The ruler's Shift+drag, driven the way a browser drives it: a pointerdown the hook either claims
// or declines, then window pointermove/pointerup. The track is a real element with a stubbed box
// (jsdom lays nothing out), so the ms the hook reports is the real x-to-ms mapping, not a fixture.
const DUR = 10_000;
const track: { current: HTMLDivElement | null } = { current: null };
const box = (left: number, width: number) => ({ left, width, top: 0, right: left + width, bottom: 0, height: 0, x: left, y: 0, toJSON: () => ({}) }) as DOMRect;

let api: { begin: (e: { clientX: number; shiftKey: boolean; preventDefault(): void; stopPropagation(): void }) => boolean; range: Range | null };
let root: Root, container: HTMLDivElement;

function Harness() {
  const [range, setRange] = useState<Range | null>(null);
  const begin = useRangeSelect({ dur: DUR, trackRef: track, range, setRange });
  api = { begin, range };
  return null;
}

/** A pointerdown on the ruler; returns whether the range gesture claimed it. */
const down = (clientX: number, shiftKey: boolean) => {
  let claimed = false;
  act(() => { claimed = api.begin({ clientX, shiftKey, preventDefault: () => {}, stopPropagation: () => {} }); });
  return claimed;
};
const at = (type: string, clientX: number) => act(() => { window.dispatchEvent(new MouseEvent(type, { clientX, bubbles: true })); });
const esc = () => act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })); });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  const el = document.createElement("div");
  el.getBoundingClientRect = () => box(0, 1000);
  track.current = el;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  act(() => { root.render(createElement(Harness)); });
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("rangeOf", () => {
  it("orders the pair and clamps it into the clip", () => {
    expect(rangeOf(600, 200, box(0, 1000), DUR)).toEqual([2000, 6000]);
    expect(rangeOf(-50, 1200, box(0, 1000), DUR)).toEqual([0, 10_000]);
  });
});

describe("useRangeSelect", () => {
  it("a Shift+drag selects an ordered range in clip ms, whichever way it is dragged", () => {
    expect(down(600, true)).toBe(true);
    at("pointermove", 200);
    at("pointerup", 200);
    expect(api.range).toEqual([2000, 6000]); // dragged right to left, reported low to high
  });

  it("a plain drag is declined, so the ruler scrubs and no range appears", () => {
    expect(down(600, false)).toBe(false);
    at("pointermove", 200);
    at("pointerup", 200);
    expect(api.range).toBeNull();
  });

  it("a Shift+CLICK (no movement past the drag threshold) leaves no zero-width range behind", () => {
    down(600, true);
    at("pointerup", 601);
    expect(api.range).toBeNull();
  });

  it("Escape clears the range", () => {
    down(300, true);
    at("pointermove", 700);
    at("pointerup", 700);
    expect(api.range).toEqual([3000, 7000]);
    esc();
    expect(api.range).toBeNull();
  });
});
