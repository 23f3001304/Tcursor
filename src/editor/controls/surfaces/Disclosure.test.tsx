// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { Disclosure, readDisclosure, writeDisclosure } from "./Disclosure";

let root: Root, container: HTMLDivElement;
const head = () => container.querySelector<HTMLButtonElement>(".e-more-btn")!;
const body = () => container.querySelector(".e-more-inner");

const show = () =>
  act(() => {
    root.render(
      <Disclosure id="panelx">
        <p>hidden row</p>
      </Disclosure>,
    );
  });

beforeEach(() => {
  (globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  localStorage.clear();
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});
afterEach(() => {
  act(() => {
    root.unmount();
  });
  container.remove();
  localStorage.clear();
});

describe("readDisclosure / writeDisclosure", () => {
  it("only ever reports open for the exact stored flag", () => {
    expect(readDisclosure("panelx")).toBe(false);
    writeDisclosure("panelx", true);
    expect(readDisclosure("panelx")).toBe(true);
    writeDisclosure("panelx", false);
    expect(readDisclosure("panelx")).toBe(false);
  });

  it("keys panels apart, so one panel's More does not open another's", () => {
    writeDisclosure("background", true);
    expect(readDisclosure("cursor")).toBe(false);
  });
});

describe("Disclosure", () => {
  it("starts closed, with its content out of the tree (and so out of the tab order)", () => {
    show();
    expect(head().getAttribute("aria-expanded")).toBe("false");
    expect(body()).toBeNull();
    expect(container.textContent).not.toContain("hidden row");
  });

  it("opens on click, revealing its children", () => {
    show();
    act(() => {
      head().click();
    });
    expect(head().getAttribute("aria-expanded")).toBe("true");
    expect(body()?.textContent).toBe("hidden row");
  });

  it("closes again on a second click", () => {
    show();
    act(() => {
      head().click();
    });
    act(() => {
      head().click();
    });
    expect(head().getAttribute("aria-expanded")).toBe("false");
  });

  it("remembers the state it was left in, per panel", () => {
    show();
    act(() => {
      head().click();
    });
    expect(localStorage.getItem("tcursor.panel.more.panelx")).toBe("1");
    act(() => {
      root.unmount();
    });
    root = createRoot(container);
    show();
    expect(head().getAttribute("aria-expanded")).toBe("true");
    expect(body()?.textContent).toBe("hidden row");

    act(() => {
      head().click();
    });
    expect(localStorage.getItem("tcursor.panel.more.panelx")).toBe("0");
  });

  it("labels itself More unless the caller says otherwise", () => {
    show();
    expect(head().textContent).toBe("More");
  });
});
