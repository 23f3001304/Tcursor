// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import type { Caption } from "../../../shared/edit";
import { CaptionList } from "./CaptionList";

const cap = (id: string, start_ms: number, end_ms: number, text: string): Caption => ({
  id,
  start_ms,
  end_ms,
  text,
  words: [],
});
const CAPS = [cap("c0", 0, 1000, "first line"), cap("c1", 1000, 2000, "second line")];

let root: Root, container: HTMLDivElement;
const rows = () => [...container.querySelectorAll<HTMLElement>(".e-caplist-row")];
const draw = (timeMs: number, sel: string | null) =>
  act(() => {
    root.render(<CaptionList captions={CAPS} timeMs={timeMs} sel={sel} onPick={() => {}} />);
  });

beforeEach(() => {
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  Element.prototype.scrollIntoView = () => {};
});
afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

describe("the transcript list", () => {
  it("lights the row the playhead is inside and moves the mark as it plays", () => {
    draw(500, null);
    expect(rows().map((r) => r.className.includes("live"))).toEqual([true, false]);
    draw(1500, null);
    expect(rows().map((r) => r.className.includes("live"))).toEqual([false, true]);
  });

  it("stacks the live mark and the selected mark on one row without dropping either", () => {
    draw(500, "c0");
    const [first] = rows();
    expect(first.className).toContain("live");
    expect(first.className).toContain("on");
    expect(first.getAttribute("aria-current")).toBe("true");
    expect(first.getAttribute("aria-pressed")).toBe("true");
  });

  it("shows the whole line in the row rather than behind a hover tooltip", () => {
    draw(0, null);
    const [first] = rows();
    expect(first.getAttribute("title")).toBeNull();
    expect(first.querySelector(".e-captxt")?.textContent).toBe("first line");
  });
});
