import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { CategorySection, defaultOpenIndex, readCategory, writeCategory } from "./CategorySection";

let root: Root, container: HTMLDivElement;
const head = () => container.querySelector<HTMLButtonElement>(".e-cat-btn")!;
const body = () => container.querySelector(".e-cat-inner");
const sel = () => container.querySelector(".e-cat-sel")?.textContent ?? null;

const show = (defaultOpen: boolean, selectedName: string | null = null) => act(() => {
  root.render(
    <CategorySection id="playful" label="Playful" count={4} selectedName={selectedName} defaultOpen={defaultOpen}>
      <p>Cat</p>
    </CategorySection>,
  );
});

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  localStorage.clear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => { act(() => { root.unmount(); }); container.remove(); localStorage.clear(); });

describe("readCategory / writeCategory", () => {
  it("reports null until the user has actually toggled the section", () => {
    // Not `false`: "never touched" has to be distinguishable from "closed on purpose", or a
    // section could never follow the selection after the first mount.
    expect(readCategory("playful")).toBeNull();
    writeCategory("playful", true);
    expect(readCategory("playful")).toBe(true);
    writeCategory("playful", false);
    expect(readCategory("playful")).toBe(false);
  });

  it("keys sections apart, so opening one category does not open another", () => {
    writeCategory("classic", true);
    expect(readCategory("playful")).toBeNull();
  });
});

describe("defaultOpenIndex", () => {
  it("opens the section holding the selection", () => {
    expect(defaultOpenIndex([false, false, true, false])).toBe(2);
  });

  it("falls back to the first section when nothing holds the selection", () => {
    expect(defaultOpenIndex([false, false, false])).toBe(0);
    expect(defaultOpenIndex([])).toBe(0);
  });

  it("opens the FIRST holder if a caller ever passes two", () => {
    expect(defaultOpenIndex([false, true, true])).toBe(1);
  });
});

describe("CategorySection", () => {
  it("starts closed when told to, with its content out of the tree (and so out of the tab order)", () => {
    show(false);
    expect(head().getAttribute("aria-expanded")).toBe("false");
    expect(body()).toBeNull();
    expect(container.textContent).not.toContain("Cat");
  });

  it("starts open when it is the section holding the selection", () => {
    show(true);
    expect(head().getAttribute("aria-expanded")).toBe("true");
    expect(body()?.textContent).toBe("Cat");
  });

  it("names its label and its item count in the header", () => {
    show(true);
    expect(container.querySelector(".e-cat-label")?.textContent).toBe("Playful");
    expect(container.querySelector(".e-cat-count")?.textContent).toBe("4");
  });

  it("shows the chosen item's name at the right only while closed", () => {
    show(false, "Cat");
    expect(sel()).toBe("Cat");
    act(() => { head().click(); });
    expect(sel()).toBeNull(); // open, the ring on the tile says it better
  });

  it("says nothing at the right when the selection is in another section", () => {
    show(false, null);
    expect(sel()).toBeNull();
  });

  it("opens and closes on click, and remembers what it was left in", () => {
    show(false);
    act(() => { head().click(); });
    expect(head().getAttribute("aria-expanded")).toBe("true");
    expect(localStorage.getItem("tcursor.panel.cat.playful")).toBe("1");
    act(() => { head().click(); });
    expect(head().getAttribute("aria-expanded")).toBe("false");
    expect(localStorage.getItem("tcursor.panel.cat.playful")).toBe("0");
  });

  it("lets a remembered state beat the default, in both directions", () => {
    writeCategory("playful", true);
    show(false);
    expect(head().getAttribute("aria-expanded")).toBe("true");
    act(() => { root.unmount(); });
    root = createRoot(container);
    writeCategory("playful", false);
    show(true);
    expect(head().getAttribute("aria-expanded")).toBe("false");
  });

  it("writes nothing until the user toggles, so an untouched section keeps following the selection", () => {
    show(true);
    expect(localStorage.getItem("tcursor.panel.cat.playful")).toBeNull();
  });

  it("wires the header to the body it controls", () => {
    show(true);
    expect(head().getAttribute("aria-controls")).toBe(container.querySelector(".e-cat-body")?.id);
  });
});
