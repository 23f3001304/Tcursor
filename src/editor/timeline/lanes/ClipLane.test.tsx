// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditOp } from "../../../shared/edit";
import { clipsFixtureMap } from "../../../shared/math/remap.fixture";
import { ClipLane } from "./ClipLane";

const clips = [
  { id: "cl1", src_in_ms: 6000, src_out_ms: 9000, transition_in_ms: 0 },
  { id: "cl0", src_in_ms: 500, src_out_ms: 4000, transition_in_ms: 500 },
];

const track: { current: HTMLDivElement | null } = { current: null };
let ops: EditOp[] = [];
let sels: (string | null)[] = [];
let root: Root, container: HTMLDivElement;

const q = (s: string) => container.querySelector<HTMLElement>(s);
const hasLabel = (t: string) =>
  [...container.querySelectorAll<HTMLElement>(".e-zlabel")].some((e) => e.textContent === t);
const at = (el: Element | null, type: string, clientX: number) =>
  act(() => {
    el?.dispatchEvent(new MouseEvent(type, { clientX, clientY: 0, bubbles: true }));
  });
const onWindow = (type: string, clientX: number) =>
  act(() => {
    window.dispatchEvent(new MouseEvent(type, { clientX, clientY: 0, bubbles: true }));
  });

const mount = (sel: string | null = null) =>
  act(() => {
    root.render(
      <ClipLane
        clips={clips}
        map={clipsFixtureMap()}
        dur={10_000}
        sel={sel}
        onSel={(id) => sels.push(id)}
        onApply={async (op) => {
          ops.push(op);
          return null;
        }}
        track={track}
      />,
    );
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops = [];
  sels = [];
  container = document.createElement("div");
  document.body.appendChild(container);
  const el = document.createElement("div");
  el.getBoundingClientRect = () =>
    ({
      left: 0,
      width: 1000,
      top: 0,
      right: 1000,
      bottom: 0,
      height: 0,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }) as DOMRect;
  track.current = el;
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
});

describe("ClipLane", () => {
  it("draws one pill per clip, numbered by output position", () => {
    mount();
    expect(hasLabel("1 - 5.0s")).toBe(true);
    expect(hasLabel("2 - 2.0s")).toBe(true);
  });

  it("marks the selected clip", () => {
    mount("cl0");
    const pill = q('[data-region-id="cl0"]');
    expect(pill?.className).toContain("sel");
  });

  it("puts each pill on its source range, not on its output position", () => {
    mount();
    const first = q('[data-region-id="cl1"]');
    expect(first?.style.left).toBe("60%");
  });

  it("dragging a pill's body onto an earlier clip's position moves it to the front", () => {
    mount();
    const pill = q('[data-region-id="cl0"]');
    at(pill, "pointerdown", 0);
    onWindow("pointermove", 600);
    onWindow("pointerup", 600);
    expect(ops).toEqual([{ op: "move_clip", id: "cl0", to_index: 0 }]);
  });

  it("dragging a pill's body onto a later clip's position moves it to the back", () => {
    mount();
    const pill = q('[data-region-id="cl1"]');
    at(pill, "pointerdown", 1000);
    onWindow("pointermove", 450);
    onWindow("pointerup", 450);
    expect(ops).toEqual([{ op: "move_clip", id: "cl1", to_index: 1 }]);
  });

  it("dragging the left edge commits update_clip with the new src_in_ms and the untouched src_out_ms", () => {
    mount();
    const handles = q('[data-region-id="cl0"]')!.querySelectorAll(".e-zh");
    at(handles[0], "pointerdown", 0);
    onWindow("pointermove", 80);
    onWindow("pointerup", 80);
    expect(ops).toEqual([{ op: "update_clip", id: "cl0", src_in_ms: 1300, src_out_ms: 4000 }]);
  });

  it("dragging the right edge past the 100ms floor clamps instead of crossing the left edge", () => {
    mount();
    const handles = q('[data-region-id="cl1"]')!.querySelectorAll(".e-zh");
    at(handles[1], "pointerdown", 1000);
    onWindow("pointermove", 500);
    onWindow("pointerup", 500);
    expect(ops).toEqual([{ op: "update_clip", id: "cl1", src_in_ms: 6000, src_out_ms: 6100 }]);
  });

  it("a press and release under the drag threshold selects but sends no op", () => {
    mount();
    const pill = q('[data-region-id="cl0"]');
    at(pill, "pointerdown", 0);
    onWindow("pointerup", 1);
    expect(ops).toEqual([]);
    expect(sels).toEqual(["cl0"]);
  });

  it("a body drag past the threshold that lands over no other clip sends no op", () => {
    mount();
    const pill = q('[data-region-id="cl0"]');
    at(pill, "pointerdown", 0);
    onWindow("pointermove", 10);
    onWindow("pointerup", 10);
    expect(ops).toEqual([]);
  });

  it("an edge drag past the threshold that lands back on the original bound sends no op", () => {
    mount();
    const handles = q('[data-region-id="cl0"]')!.querySelectorAll(".e-zh");
    at(handles[0], "pointerdown", 0);
    onWindow("pointermove", 0);
    onWindow("pointerup", 10);
    expect(ops).toEqual([]);
  });
});
