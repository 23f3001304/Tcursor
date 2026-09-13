import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { EditDoc, EditOp } from "../../lib/edit";
import { TimeLane } from "./TimeLane";

// The Time lane's two jobs: say what a speed span does, and let it be retimed by its edge handles
// (the pill idiom every other lane already has, through `useRegionDrag`). The track is a real
// element with a stubbed box, so the drag's px-to-ms conversion is the real one.
const DUR = 10_000;
const DOC = { speed: [{ id: "s0", start_ms: 1000, end_ms: 3000, factor: 2 }], cuts: [] } as unknown as EditDoc;

const track: { current: HTMLDivElement | null } = { current: null };
let ops: EditOp[] = [];
let root: Root, container: HTMLDivElement;

const q = (s: string) => container.querySelector<HTMLElement>(s);
const all = (s: string) => container.querySelectorAll(s);
const at = (el: Element | null, type: string, clientX: number) =>
  act(() => { el?.dispatchEvent(new MouseEvent(type, { clientX, clientY: 0, bubbles: true })); });
const onWindow = (type: string, clientX: number) =>
  act(() => { window.dispatchEvent(new MouseEvent(type, { clientX, clientY: 0, bubbles: true })); });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  ops = [];
  container = document.createElement("div");
  document.body.appendChild(container);
  const el = document.createElement("div");
  el.getBoundingClientRect = () => ({ left: 0, width: 1000, top: 0, right: 1000, bottom: 0, height: 0, x: 0, y: 0, toJSON: () => ({}) }) as DOMRect;
  track.current = el;
  root = createRoot(container);
  act(() => {
    root.render(
      <TimeLane doc={DOC} dur={DUR} sel={null} onSel={() => {}} track={track}
        onApply={async (op) => { ops.push(op); return null; }} />
    );
  });
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("TimeLane", () => {
  it("renders one pill per speed span, labelled with its factor", () => {
    expect(all(".e-spdblk")).toHaveLength(1);
    expect(q(".e-spdblk .e-zlabel")?.textContent).toBe("2x");
    expect(q(".e-spdblk")?.style.left).toBe("10%");
    expect(q(".e-spdblk")?.style.width).toBe("20%");
  });

  it("dragging the right edge commits update_speed with the new end and the untouched start", () => {
    const handles = all(".e-spdblk .e-zh");
    expect(handles).toHaveLength(2); // left and right, the shared RegionRows markup
    at(handles[1], "pointerdown", 300);
    onWindow("pointermove", 500);   // +200px of a 1000px track = +2000ms
    onWindow("pointerup", 500);
    expect(ops).toEqual([{ op: "update_speed", id: "s0", start_ms: 1000, end_ms: 5000 }]);
  });

  it("a bare click on a handle selects without committing a no-op retime", () => {
    const handles = all(".e-spdblk .e-zh");
    at(handles[1], "pointerdown", 300);
    onWindow("pointerup", 301);
    expect(ops).toEqual([]);
  });
});
