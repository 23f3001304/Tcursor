import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Segmented, segmentedNextIndex } from "./Segmented";

describe("segmentedNextIndex", () => {
  it("steps forward and back, wrapping at both ends", () => {
    expect(segmentedNextIndex("ArrowRight", 0, 3)).toBe(1);
    expect(segmentedNextIndex("ArrowRight", 2, 3)).toBe(0);
    expect(segmentedNextIndex("ArrowLeft", 1, 3)).toBe(0);
    expect(segmentedNextIndex("ArrowLeft", 0, 3)).toBe(2);
  });

  it("treats the vertical arrows the same as the horizontal ones", () => {
    expect(segmentedNextIndex("ArrowDown", 0, 3)).toBe(1);
    expect(segmentedNextIndex("ArrowUp", 0, 3)).toBe(2);
  });

  it("jumps to the ends on Home/End and ignores everything else", () => {
    expect(segmentedNextIndex("Home", 2, 3)).toBe(0);
    expect(segmentedNextIndex("End", 0, 3)).toBe(2);
    expect(segmentedNextIndex("Enter", 0, 3)).toBeNull();
    expect(segmentedNextIndex("ArrowRight", 0, 0)).toBeNull();
  });

  it("starts from the first segment when nothing matches the value yet", () => {
    expect(segmentedNextIndex("ArrowRight", -1, 3)).toBe(1);
  });
});

let root: Root, container: HTMLDivElement;
const radios = () => Array.from(container.querySelectorAll<HTMLButtonElement>('[role="radio"]'));

let picked: string[] = [];
const OPTS = [
  { value: "system", label: "System" },
  { value: "enhanced", label: "Enhanced" },
  { value: "hidden", label: "Hidden" },
];
const show = (value: string) => act(() => {
  root.render(<Segmented value={value} options={OPTS} ariaLabel="Cursor style"
    onChange={(v) => picked.push(v)} />);
});

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  picked = [];
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); });

describe("Segmented", () => {
  it("is a radio group with exactly one checked segment", () => {
    show("enhanced");
    expect(container.querySelector('[role="radiogroup"]')?.getAttribute("aria-label")).toBe("Cursor style");
    expect(radios().map((b) => b.textContent)).toEqual(["System", "Enhanced", "Hidden"]);
    expect(radios().filter((b) => b.getAttribute("aria-checked") === "true")).toHaveLength(1);
    expect(radios()[1].getAttribute("aria-checked")).toBe("true");
  });

  it("picks the segment that was clicked", () => {
    show("system");
    act(() => { radios()[2].click(); });
    expect(picked).toEqual(["hidden"]);
  });

  it("moves the selection with the arrow keys, wrapping past the last segment", () => {
    show("system");
    act(() => { radios()[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })); });
    expect(picked).toEqual(["enhanced"]);
    show("hidden");
    act(() => { radios()[2].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })); });
    expect(picked).toEqual(["enhanced", "system"]);
  });

  it("keeps only the chosen segment in the tab order, as a radio group should", () => {
    show("hidden");
    expect(radios().map((b) => b.tabIndex)).toEqual([-1, -1, 0]);
  });
});
